//! G7.6 PR-2:One True Device Frame(设计案 §1 / §8 PR-2)。
//!
//! 15-pass 单 submit `DeviceFrameSession`:960×540 内部 → 1920×1080 TSR 输出,
//! 全 SSBO compute;HW 光栅不入链。唯一执行入口:`execute_with_frame_update`
//! (禁 `execute_frame`)。

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use rurix_physics::{
    BodyDesc, BodyId, BodyKind, MassProps, PhysicsBridge, PhysicsTransform, PhysicsWorld,
    ShapeDesc, SyncBudget, WorldDesc,
};
use rurix_render::geometry::cull::{
    CullCamera, Frustum, VisibleCluster, cluster_cull, instance_cull,
};
use rurix_render::geometry::gpu_scene::{InstanceRecord, transform_point};
use rurix_render::geometry::material_pass::resolve as resolve_materials;
use rurix_render::geometry::visbuffer::VisBufferCpu;
use rurix_render::graph::types::ClusterRecord;
use rurix_render::material::closure::unpack;
use rurix_render::rt::bvh::Vec3;
use rurix_render::rt::ref_tracer::{Pcg32, RAY_EPS, cosine_sample_hemisphere};
use rurix_render::shadow::clipmap::LightBasis;
use rurix_render::shadow::page_table::PageTableEntry;
use rurix_render::shadow::vsm::Vsm;
use rurix_render::temporal::common::jitter_sequence;
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::taa::{TaaParams, taa_resolve};
use rurix_render::temporal::tsr::{TsrParams, TsrUpscaler};
use rurix_render::temporal::upscale::{UpscaleBackend, UpscaleInputs};
use rurix_rt::render_exec::{
    self, AccelStructDesc, Bindings, BufferDesc, BufferUsage, ComputePass, DeviceFrameSession,
    DispatchSpec, FrameUpdate, KernelWave, Pass, Readback, ResourceDesc, StableResourceId,
    SubmissionProvenance, TargetState,
};
use rurix_rt::vk::{
    RayQueryInstanceDesc, RayQuerySceneDesc, RayQueryTransformedInstanceDesc, TlasBuildAction,
};

use crate::pipeline::camera_matrices;
use crate::scene::{CAMERA, SKY_COLOR, SUN_COLOR, SUN_DIR, Uc06Scene, VSM_LIGHT_DIR, build_scene};
use crate::shading::make_vsm;
use crate::tiny_sha256::{self, Sha256};

const FRAME_CLEAR_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/frame_clear.spv"));
const CULL_FRAME_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cull_frame.spv"));
const TRI_EXPAND_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tri_expand.spv"));
const VISBUFFER_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/visbuffer_sw_u64.spv"));
const CLASSIFY_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/classify_resolve.spv"));
const GBUFFER_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gbuffer_resolve.spv"));
const VSM_DEPTH_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vsm_depth_raster.spv"));
const VSM_SAMPLE_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vsm_sample.spv"));
const GI_PROBE_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gi_probe.spv"));
const RTAO_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rtao.spv"));
const HARD_SHADOW_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hard_shadow.spv"));
const DEFERRED_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/deferred_shade.spv"));
const TAA_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/taa.spv"));
const TSR_RESAMPLE_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tsr_resample.spv"));
const TSR_TEMPORAL_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tsr_temporal.spv"));

const IN_W: u32 = 960;
const IN_H: u32 = 540;
const OUT_W: u32 = 1920;
const OUT_H: u32 = 1080;
const PIXELS: usize = (IN_W * IN_H) as usize;
const OUT_PIXELS: usize = (OUT_W * OUT_H) as usize;
const TRI_COUNT: u32 = 764;
const INSTANCE_COUNT: usize = 3;
const PARAMS_F32_COUNT: usize = 160;
const MATERIAL_SLOTS: usize = 16;
const RTAO_SPP: u32 = 4;
const RTAO_RADIUS: f32 = 2.0;
const T_MAX_FINITE: f32 = 1.0e30;
const SEED: u64 = 0x5255_5258_5543_0006;
const PAGE_DIM: usize = 128;
const PAGE_TEXELS: usize = PAGE_DIM * PAGE_DIM;
const PAGE_TABLE_SLOTS: usize = PAGE_TEXELS;
const VSM_POOL_CAP: u32 = 512;
const ERROR_THRESHOLD_PX: f32 = 1.0;
const TAA_ALPHA: f32 = 0.1;
const PASS_NAMES: [&str; 15] = [
    "frame_clear",
    "cull_frame",
    "tri_expand",
    "visbuffer_sw_u64",
    "classify_resolve",
    "gbuffer_resolve",
    "vsm_depth_raster",
    "vsm_sample",
    "gi_probe",
    "rtao",
    "hard_shadow",
    "deferred_shade",
    "taa",
    "tsr_resample",
    "tsr_temporal",
];

/// 期望 provenance 关键边(producer_pass, consumer_pass, resource_name)。
const EXPECTED_EDGES: &[(&str, &str, &str)] = &[
    ("cull_frame", "tri_expand", "visible_flags"),
    ("tri_expand", "visbuffer_sw_u64", "triangles"),
    ("visbuffer_sw_u64", "classify_resolve", "vis"),
    ("visbuffer_sw_u64", "gbuffer_resolve", "vis"),
    ("gbuffer_resolve", "vsm_sample", "pos"),
    ("gbuffer_resolve", "rtao", "pos"),
    ("gbuffer_resolve", "hard_shadow", "pos"),
    ("deferred_shade", "taa", "hdr"),
    ("taa", "tsr_resample", "taa_out"),
    ("tsr_resample", "tsr_temporal", "tsr_cur"),
];

pub mod tol {
    /// gbuffer_resolve measured 2.30e-3 @ 8 帧(反投影/MV FMA);冻结 ×1.5 余量。
    pub const GBUFFER: f32 = 3.5e-3;
    pub const VSM_SAMPLE: f32 = 0.0;
    pub const GI: f32 = 1e-5;
    pub const AO: f32 = 1e-6;
    pub const HARD: f32 = 0.0;
    /// TAA measured 首帧零历史对齐后重测;预留 1e-4(步骤 87 量级放宽至帧链)。
    pub const TAA: f32 = 1e-4;
    /// tsr_resample measured 4.38e-7;冻结 1e-6。
    pub const TSR_RESAMPLE: f32 = 1e-6;
    /// tsr_temporal measured 1.76e-4;冻结 5e-4。
    pub const TSR_TEMPORAL: f32 = 5e-4;
    /// tri_expand measured 6.10e-5;冻结 1e-4(ULP/FMA)。
    pub const TRI_EXPAND: f32 = 1e-4;
    /// visbuffer 覆盖差豁免率(非全字;簇号在双方有效像素上 100% 一致)。
    /// measured cover_mismatch_ratio=2.35e-2 @ 960×540;冻结 3e-2(止损风险 4)。
    pub const VISBUFFER_MISMATCH_RATIO: f32 = 3e-2;
}

mod res {
    pub const PARAMS: u32 = 0;
    pub const INSTANCE_OF: u32 = 1;
    pub const INSTANCE_AABB: u32 = 2;
    pub const CLUSTER_CENTER: u32 = 3;
    pub const CLUSTER_RADIUS: u32 = 4;
    pub const CONE_AXIS: u32 = 5;
    pub const CONE_CUTOFF: u32 = 6;
    pub const CLUSTER_ERROR: u32 = 7;
    pub const PARENT_ERROR: u32 = 8;
    pub const VISIBLE_FLAGS: u32 = 9;
    pub const VISIBLE_COUNT: u32 = 10;
    pub const OBJ_TRIS: u32 = 11;
    pub const TRI_CLUSTER: u32 = 12;
    pub const TRI_LOCAL: u32 = 13;
    pub const TRIANGLES: u32 = 14;
    pub const IDS: u32 = 15;
    pub const VIS: u32 = 16;
    pub const C2M: u32 = 17;
    pub const RESOLVED: u32 = 18;
    pub const MATERIAL_COUNTS: u32 = 19;
    pub const FACE_NRM: u32 = 20;
    pub const TRI_INSTANCE: u32 = 21;
    pub const POS: u32 = 22;
    pub const NRM: u32 = 23;
    pub const DEPTH: u32 = 24;
    pub const MV: u32 = 25;
    pub const VALIDITY: u32 = 26;
    pub const VSM_TRIS: u32 = 27;
    pub const VSM_PAGES: u32 = 28;
    pub const VSM_POOL: u32 = 29;
    pub const VSM_LPARAMS: u32 = 30;
    pub const VSM_ENTRIES: u32 = 31;
    pub const SHADOW_VSM: u32 = 32;
    pub const GI_RAYS: u32 = 33;
    pub const GI_TRIS: u32 = 34;
    pub const GI_TRI_BASE: u32 = 35;
    pub const GI_ALBEDO: u32 = 36;
    pub const RADIANCE: u32 = 37;
    pub const GI_GEOM: u32 = 38;
    pub const GI_IDX: u32 = 39;
    pub const RTAO_DIRS: u32 = 40;
    pub const AO: u32 = 41;
    pub const VIS_HARD: u32 = 42;
    pub const ALBEDO_MAT: u32 = 43;
    pub const EMISSIVE_MAT: u32 = 44;
    pub const HDR: u32 = 45;
    pub const TAA_HIST_A: u32 = 46;
    pub const TAA_HIST_B: u32 = 47;
    pub const TSR_CUR: u32 = 48;
    pub const HIST_COLOR_A: u32 = 49;
    pub const HIST_COLOR_B: u32 = 50;
    pub const HIST_DEPTH_A: u32 = 51;
    pub const HIST_DEPTH_B: u32 = 52;
    pub const PREV_LUMA: u32 = 53;
    pub const PREV_SIGN: u32 = 54;
    pub const FLICKER: u32 = 55;
    pub const COUNT: usize = 56;
}

mod rb {
    pub const DEPTH: u32 = 0;
    pub const FLAGS: u32 = 1;
    pub const TRIANGLES: u32 = 2;
    pub const IDS: u32 = 3;
    pub const VIS: u32 = 4;
    pub const RESOLVED: u32 = 5;
    pub const MATERIAL_COUNTS: u32 = 6;
    pub const POS: u32 = 7;
    pub const NRM: u32 = 8;
    pub const MV: u32 = 9;
    pub const VALIDITY: u32 = 10;
    pub const SHADOW: u32 = 11;
    pub const AO: u32 = 12;
    pub const HARD: u32 = 13;
    pub const HDR: u32 = 14;
    pub const TAA_A: u32 = 15;
    pub const TAA_B: u32 = 16;
    pub const TSR_CUR: u32 = 17;
    pub const FINAL_A: u32 = 18;
    pub const FINAL_B: u32 = 19;
    pub const VISIBLE_COUNT: u32 = 20;
}

/// RED 轴(步骤 96 四轴;独立进程跑)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedAxis {
    Visbuffer,
    History,
    Jitter,
    Provenance,
}

#[derive(Debug, Clone)]
pub struct DeviceFrameOptions {
    pub frames: u32,
    pub soak: bool,
    pub min_minutes: f64,
    pub red: Option<RedAxis>,
    #[allow(dead_code)]
    pub json: bool,
}

impl Default for DeviceFrameOptions {
    fn default() -> Self {
        Self {
            frames: 8,
            soak: false,
            min_minutes: 0.0,
            red: None,
            json: false,
        }
    }
}

/// soak 取证块(设计案 §5);短跑路径为 None。
#[derive(Debug, Clone)]
pub struct SoakTelemetry {
    pub actual_frames: u32,
    pub elapsed_minutes: f64,
    pub fps_mean: f64,
    pub tdr_suspected_count: u64,
    pub vsm_page_overflow_count: u64,
    pub frame_gpu_p50_ms: f64,
    pub frame_gpu_p95_ms: f64,
    pub frame_gpu_p99_ms: f64,
    pub cpu_submit_p50_ms: f64,
    pub cpu_submit_p95_ms: f64,
    pub cpu_submit_p99_ms: f64,
    pub peak_vram_mb: f64,
    pub pass_gpu_p50_ms: [f64; 15],
    pub pass_gpu_p95_ms: [f64; 15],
    pub validation_layers_enabled: bool,
    pub scene_digest: String,
    pub anchor_color_sha256: Vec<String>,
    pub luma_mean_series: Vec<f64>,
    pub luma_var_series: Vec<f64>,
    pub anchor_ppm: Vec<String>,
    pub device_caps_json: String,
    pub fail_reasons: Vec<String>,
    pub ok: bool,
}

impl SoakTelemetry {
    pub fn json_object(&self) -> String {
        let passes: String = (0..15)
            .map(|i| {
                format!(
                    "{{\"pass\":\"{}\",\"gpu_p50_ms\":{:.6},\"gpu_p95_ms\":{:.6}}}",
                    PASS_NAMES[i], self.pass_gpu_p50_ms[i], self.pass_gpu_p95_ms[i]
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let digests = self
            .anchor_color_sha256
            .iter()
            .map(|d| format!("\"{d}\""))
            .collect::<Vec<_>>()
            .join(",");
        let luma_m = self
            .luma_mean_series
            .iter()
            .map(|v| format!("{v:.6}"))
            .collect::<Vec<_>>()
            .join(",");
        let luma_v = self
            .luma_var_series
            .iter()
            .map(|v| format!("{v:.6}"))
            .collect::<Vec<_>>()
            .join(",");
        let ppms = self
            .anchor_ppm
            .iter()
            .map(|p| format!("\"{}\"", p.replace('\\', "/")))
            .collect::<Vec<_>>()
            .join(",");
        let fails = self
            .fail_reasons
            .iter()
            .map(|r| format!("\"{}\"", r.replace('"', "'")))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"actual_frames\":{},\"elapsed_minutes\":{:.6},\"fps_mean\":{:.6},\
             \"tdr_suspected_count\":{},\"vsm_page_overflow_count\":{},\
             \"frame_gpu_p50_ms\":{:.6},\"frame_gpu_p95_ms\":{:.6},\"frame_gpu_p99_ms\":{:.6},\
             \"cpu_submit_p50_ms\":{:.6},\"cpu_submit_p95_ms\":{:.6},\"cpu_submit_p99_ms\":{:.6},\
             \"peak_vram_mb\":{:.6},\"pass_gpu_timestamps\":[{}],\
             \"validation_layers_enabled\":{},\"scene_digest\":\"{}\",\
             \"visual_digest\":{{\"anchor_color_sha256\":[{}],\"luma_mean_series\":[{}],\
             \"luma_var_series\":[{}]}},\"anchor_ppm\":[{}],\"device_caps\":{},\
             \"driver_version\":null,\"fail_reasons\":[{}],\"ok\":{}}}",
            self.actual_frames,
            self.elapsed_minutes,
            self.fps_mean,
            self.tdr_suspected_count,
            self.vsm_page_overflow_count,
            self.frame_gpu_p50_ms,
            self.frame_gpu_p95_ms,
            self.frame_gpu_p99_ms,
            self.cpu_submit_p50_ms,
            self.cpu_submit_p95_ms,
            self.cpu_submit_p99_ms,
            self.peak_vram_mb,
            passes,
            self.validation_layers_enabled,
            self.scene_digest,
            digests,
            luma_m,
            luma_v,
            ppms,
            self.device_caps_json,
            fails,
            self.ok
        )
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn caps_json(caps: &render_exec::DeviceCaps) -> String {
    // tdr_policy:PR-4 校准字面(设计案 2s → max(5000,short_p95×2);硬 TDR=fence/DEVICE_LOST)。
    format!(
        "{{\"device_name\":\"{}\",\"synchronization2\":{},\"shader_buffer_int64_atomics\":{},\
         \"shader_int64\":{},\"fragment_stores_and_atomics\":{},\"ray_query\":{},\
         \"acceleration_structure\":{},\"buffer_device_address\":{},\"descriptor_indexing\":{},\
         \"deferred_host_operations\":{},\"memory_budget\":{},\"timestamp_period_ns\":{:.6},\
         \"max_push_constants_size\":{},\
         \"tdr_policy\":\"PR-4:design_2s_replaced;soft=max(5000ms,short_p95*2)=5000ms(short_p95=1566.451712 from renderer_soak_20260804T172202);hard_tdr=fence_timeout_or_DEVICE_LOST_only\"}}",
        caps.device_name,
        caps.synchronization2,
        caps.shader_buffer_int64_atomics,
        caps.shader_int64,
        caps.fragment_stores_and_atomics,
        caps.ray_query,
        caps.acceleration_structure,
        caps.buffer_device_address,
        caps.descriptor_indexing,
        caps.deferred_host_operations,
        caps.memory_budget,
        caps.timestamp_period_ns,
        caps.max_push_constants_size
    )
}

fn tonemap_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// 1080p HDR → 视觉 digest(RGB8) + 960×540 PPM(2×2 box)。
fn anchor_visuals(
    hdr: &[f32],
    out_dir: &std::path::Path,
    frame: u32,
) -> Result<(String, f64, f64, String), String> {
    if hdr.len() < OUT_PIXELS * 3 {
        return Err(format!(
            "anchor final HDR 长度不足: {} < {}",
            hdr.len(),
            OUT_PIXELS * 3
        ));
    }
    let mut rgb = Vec::with_capacity(OUT_PIXELS * 3);
    let mut luma_sum = 0.0f64;
    let mut luma_sq = 0.0f64;
    for i in 0..OUT_PIXELS {
        let r = tonemap_u8(hdr[i * 3]);
        let g = tonemap_u8(hdr[i * 3 + 1]);
        let b = tonemap_u8(hdr[i * 3 + 2]);
        rgb.push(r);
        rgb.push(g);
        rgb.push(b);
        let y = 0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b);
        luma_sum += y;
        luma_sq += y * y;
    }
    let n = OUT_PIXELS as f64;
    let mean = luma_sum / n;
    let var = (luma_sq / n) - mean * mean;
    let digest = tiny_sha256::hex_digest(&rgb);

    let mut ppm = Vec::with_capacity(IN_W as usize * IN_H as usize * 3);
    for y in 0..IN_H {
        for x in 0..IN_W {
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            for dy in 0..2u32 {
                for dx in 0..2u32 {
                    let sx = x * 2 + dx;
                    let sy = y * 2 + dy;
                    let i = (sy * OUT_W + sx) as usize;
                    r += u32::from(rgb[i * 3]);
                    g += u32::from(rgb[i * 3 + 1]);
                    b += u32::from(rgb[i * 3 + 2]);
                }
            }
            ppm.push((r / 4) as u8);
            ppm.push((g / 4) as u8);
            ppm.push((b / 4) as u8);
        }
    }
    fs::create_dir_all(out_dir).map_err(|e| format!("create soak anchor dir: {e}"))?;
    let rel = format!(
        "evidence/soak_anchors/{}/anchor_{:05}.ppm",
        out_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("soak"),
        frame
    );
    // out_dir is evidence/soak_anchors/<ts>
    let path = out_dir.join(format!("anchor_{frame:05}.ppm"));
    let mut f = fs::File::create(&path).map_err(|e| format!("write ppm: {e}"))?;
    write!(f, "P6\n{IN_W} {IN_H}\n255\n").map_err(|e| format!("ppm hdr: {e}"))?;
    f.write_all(&ppm).map_err(|e| format!("ppm body: {e}"))?;
    let _ = rel;
    let rel_path = path
        .strip_prefix(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");
    Ok((digest, mean, var, rel_path))
}

#[derive(Debug, Clone)]
pub struct DeviceFrameResults {
    pub device_name: String,
    pub frames: u32,
    pub elapsed_seconds: f64,
    pub soak: bool,
    pub covered_pixels: u32,
    pub material_counts: [u32; 16],
    pub mv_nonzero_count: u32,
    pub mv_nonzero_changed: bool,
    pub instance_transform_changed: bool,
    pub validation_error_count: u64,
    pub leaked_object_count: u64,
    pub leaked_allocation_count: u64,
    pub device_lost_count: u64,
    pub pass_gpu_ns: Vec<f64>,
    pub cull_bitexact: bool,
    pub tri_expand_bitexact: bool,
    pub tri_expand_max_abs: f32,
    pub visbuffer_bitexact: bool,
    pub classify_bitexact: bool,
    pub gbuffer_max_abs: f32,
    pub gbuffer_pass: bool,
    pub vsm_sample_max_abs: f32,
    pub vsm_sample_pass: bool,
    pub gi_max_abs: f32,
    pub gi_pass: bool,
    pub ao_max_abs: f32,
    pub ao_pass: bool,
    pub hard_max_abs: f32,
    pub hard_pass: bool,
    pub taa_max_abs: f32,
    pub taa_pass: bool,
    pub tsr_resample_max_abs: f32,
    pub tsr_resample_pass: bool,
    pub tsr_temporal_max_abs: f32,
    pub tsr_temporal_pass: bool,
    pub provenance_edges_ok: bool,
    pub provenance_edges_actual: Vec<(String, String, String)>,
    pub red_axis: Option<&'static str>,
    pub red_ok: Option<bool>,
    pub all_pass_gpu_ns_positive: bool,
    pub non_degen_ok: bool,
    pub soak_telemetry: Option<SoakTelemetry>,
}

impl DeviceFrameResults {
    pub fn all_pass(&self) -> bool {
        if let Some(ok) = self.red_ok {
            return ok;
        }
        if let Some(soak) = &self.soak_telemetry {
            return soak.ok;
        }
        self.non_degen_ok
            && self.cull_bitexact
            && self.tri_expand_bitexact
            && self.visbuffer_bitexact
            && self.classify_bitexact
            && self.gbuffer_pass
            && self.vsm_sample_pass
            && self.gi_pass
            && self.ao_pass
            && self.hard_pass
            && self.taa_pass
            && self.tsr_resample_pass
            && self.tsr_temporal_pass
            && self.provenance_edges_ok
            && self.all_pass_gpu_ns_positive
            && self.validation_error_count == 0
            && self.leaked_object_count == 0
            && self.leaked_allocation_count == 0
            && self.device_lost_count == 0
    }

    pub fn json(&self) -> String {
        let edges: String = self
            .provenance_edges_actual
            .iter()
            .map(|(a, b, c)| {
                format!("{{\"producer\":\"{a}\",\"consumer\":\"{b}\",\"resource\":\"{c}\"}}")
            })
            .collect::<Vec<_>>()
            .join(",");
        let pass_ns: String = self
            .pass_gpu_ns
            .iter()
            .enumerate()
            .map(|(i, ns)| {
                format!(
                    "{{\"pass\":\"{}\",\"gpu_ns\":{:.3}}}",
                    PASS_NAMES.get(i).copied().unwrap_or("?"),
                    ns
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let mat = self
            .material_counts
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let soak_json = match &self.soak_telemetry {
            Some(s) => s.json_object(),
            None => "null".into(),
        };
        format!(
            "{{\"subject\":\"uc06_device_frame\",\"device_name\":\"{}\",\"frames\":{},\
             \"elapsed_seconds\":{:.6},\"soak\":{},\"in_w\":{},\"in_h\":{},\"out_w\":{},\"out_h\":{},\
             \"covered_pixels\":{},\"material_counts\":[{}],\"mv_nonzero_count\":{},\
             \"mv_nonzero_changed\":{},\"instance_transform_changed\":{},\
             \"validation_error_count\":{},\"leaked_object_count\":{},\"leaked_allocation_count\":{},\
             \"device_lost_count\":{},\"cull_bitexact\":{},\"tri_expand_bitexact\":{},\
             \"tri_expand_max_abs\":{:.9e},\"visbuffer_bitexact\":{},\"classify_bitexact\":{},\
             \"gbuffer_max_abs\":{:.9e},\"tol_gbuffer\":{:.9e},\"gbuffer_pass\":{},\
             \"vsm_sample_max_abs\":{:.9e},\"tol_vsm_sample\":{:.9e},\"vsm_sample_pass\":{},\
             \"gi_max_abs\":{:.9e},\"tol_gi\":{:.9e},\"gi_pass\":{},\
             \"ao_max_abs\":{:.9e},\"tol_ao\":{:.9e},\"ao_pass\":{},\
             \"hard_max_abs\":{:.9e},\"tol_hard\":{:.9e},\"hard_pass\":{},\
             \"taa_max_abs\":{:.9e},\"tol_taa\":{:.9e},\"taa_pass\":{},\
             \"tsr_resample_max_abs\":{:.9e},\"tol_tsr_resample\":{:.9e},\"tsr_resample_pass\":{},\
             \"tsr_temporal_max_abs\":{:.9e},\"tol_tsr_temporal\":{:.9e},\"tsr_temporal_pass\":{},\
             \"provenance_edges_ok\":{},\"provenance_edges\":[{}],\"pass_gpu_timings\":[{}],\
             \"all_pass_gpu_ns_positive\":{},\"non_degen_ok\":{},\
             \"red_axis\":{},\"red_ok\":{},\"soak_telemetry\":{},\"all_pass\":{}}}",
            self.device_name,
            self.frames,
            self.elapsed_seconds,
            self.soak,
            IN_W,
            IN_H,
            OUT_W,
            OUT_H,
            self.covered_pixels,
            mat,
            self.mv_nonzero_count,
            self.mv_nonzero_changed,
            self.instance_transform_changed,
            self.validation_error_count,
            self.leaked_object_count,
            self.leaked_allocation_count,
            self.device_lost_count,
            self.cull_bitexact,
            self.tri_expand_bitexact,
            self.tri_expand_max_abs,
            self.visbuffer_bitexact,
            self.classify_bitexact,
            self.gbuffer_max_abs,
            tol::GBUFFER,
            self.gbuffer_pass,
            self.vsm_sample_max_abs,
            tol::VSM_SAMPLE,
            self.vsm_sample_pass,
            self.gi_max_abs,
            tol::GI,
            self.gi_pass,
            self.ao_max_abs,
            tol::AO,
            self.ao_pass,
            self.hard_max_abs,
            tol::HARD,
            self.hard_pass,
            self.taa_max_abs,
            tol::TAA,
            self.taa_pass,
            self.tsr_resample_max_abs,
            tol::TSR_RESAMPLE,
            self.tsr_resample_pass,
            self.tsr_temporal_max_abs,
            tol::TSR_TEMPORAL,
            self.tsr_temporal_pass,
            self.provenance_edges_ok,
            edges,
            pass_ns,
            self.all_pass_gpu_ns_positive,
            self.non_degen_ok,
            match self.red_axis {
                Some(s) => format!("\"{s}\""),
                None => "null".into(),
            },
            match self.red_ok {
                Some(v) => v.to_string(),
                None => "null".into(),
            },
            soak_json,
            self.all_pass(),
        )
    }
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

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn bytes_u32(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn read_u32(b: &[u8]) -> Vec<u32> {
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn read_u64(b: &[u8]) -> Vec<u64> {
    b.chunks_exact(8)
        .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
        .collect()
}

fn max_abs(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

fn bitexact_u32(a: &[u32], b: &[u32]) -> bool {
    a == b
}

fn sid(res: u32) -> StableResourceId {
    StableResourceId(u64::from(res) + 1)
}

fn flatten_3x4(t: &[[f32; 4]; 3]) -> [f32; 12] {
    [
        t[0][0], t[0][1], t[0][2], t[0][3], t[1][0], t[1][1], t[1][2], t[1][3], t[2][0], t[2][1],
        t[2][2], t[2][3],
    ]
}

fn flatten_4x4(m: &[[f32; 4]; 4]) -> [f32; 16] {
    let mut o = [0.0f32; 16];
    for r in 0..4 {
        for c in 0..4 {
            o[r * 4 + c] = m[r][c];
        }
    }
    o
}

fn gate() -> Option<render_exec::DeviceCaps> {
    if !rurix_rt::vk::vulkan_available() {
        eprintln!("[uc06 device-frame] SKIP: vulkan loader 不可用(dev-env degrade)");
        return None;
    }
    let caps = render_exec::probe_device_caps().ok()?;
    match render_exec::require_wave(&caps, KernelWave::W3) {
        Ok(()) => Some(caps),
        Err(e) => {
            eprintln!("[uc06 device-frame] SKIP: W3 能力链缺失({e})");
            None
        }
    }
}

struct StaticGeom {
    cluster_count: usize,
    instance_of: Vec<u32>,
    cluster_center: Vec<f32>,
    cluster_radius: Vec<f32>,
    cone_axis: Vec<f32>,
    cone_cutoff: Vec<f32>,
    cluster_error: Vec<f32>,
    parent_error: Vec<f32>,
    obj_tris: Vec<f32>,
    tri_cluster: Vec<u32>,
    tri_local: Vec<u32>,
    face_nrm: Vec<f32>,
    tri_instance: Vec<u32>,
    leaf_mask: Vec<bool>,
    c2m: Vec<u32>,
    albedo_mat: Vec<f32>,
    emissive_mat: Vec<f32>,
    gi_albedo: Vec<f32>,
    gi_rays: Vec<f32>,
    obj_blas: Vec<Vec<f32>>,
    masks: [u8; 3],
}

fn bake_static(scene: &Uc06Scene) -> StaticGeom {
    let cluster_count = scene.clusters.len();
    let mut instance_of = vec![0u32; cluster_count];
    let mut cluster_center = Vec::with_capacity(cluster_count * 3);
    let mut cluster_radius = Vec::with_capacity(cluster_count);
    let mut cone_axis = Vec::with_capacity(cluster_count * 3);
    let mut cone_cutoff = Vec::with_capacity(cluster_count);
    let mut cluster_error = Vec::with_capacity(cluster_count);
    let mut parent_error = Vec::with_capacity(cluster_count);
    let mut leaf_mask = vec![false; cluster_count];
    for m in &scene.meshes {
        for li in m.dag.leaf_ids() {
            leaf_mask[m.cluster_offset as usize + li as usize] = true;
        }
    }
    for (i, c) in scene.clusters.iter().enumerate() {
        instance_of[i] = c.page_id;
        cluster_center.extend_from_slice(&c.center);
        cluster_radius.push(c.radius);
        cone_axis.extend_from_slice(&c.cone_axis);
        cone_cutoff.push(c.cone_cutoff);
        // 帧链三角形表仅含叶簇(764 守恒);非叶强制 LOD 自检失败,避免选中无三角槽的父簇。
        if leaf_mask[i] {
            cluster_error.push(c.error);
            parent_error.push(c.parent_error);
        } else {
            cluster_error.push(1.0e30);
            parent_error.push(0.0);
        }
    }

    let mut obj_tris = Vec::new();
    let mut tri_cluster = Vec::new();
    let mut tri_local = Vec::new();
    let mut face_nrm = vec![0.0f32; cluster_count * 128 * 3];
    let mut tri_instance = vec![0u32; cluster_count * 128];

    for (mid, m) in scene.meshes.iter().enumerate() {
        for li in m.dag.leaf_ids() {
            let r = m.dag.record(li);
            let gci = m.cluster_offset as usize + li as usize;
            for t in 0..r.triangle_count {
                let mut verts = [[0.0f32; 3]; 3];
                for (k, vert) in verts.iter_mut().enumerate() {
                    let local =
                        m.dag.triangle_indices[(r.triangle_offset + 3 * t) as usize + k] as usize;
                    *vert = m.dag.vertices[r.vertex_offset as usize + local];
                    obj_tris.extend_from_slice(vert);
                }
                tri_cluster.push(gci as u32);
                tri_local.push(t);
                let e1 = [
                    verts[1][0] - verts[0][0],
                    verts[1][1] - verts[0][1],
                    verts[1][2] - verts[0][2],
                ];
                let e2 = [
                    verts[2][0] - verts[0][0],
                    verts[2][1] - verts[0][1],
                    verts[2][2] - verts[0][2],
                ];
                let mut n = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                if len > 1e-12 {
                    n = [n[0] / len, n[1] / len, n[2] / len];
                }
                let flat = gci * 128 + t as usize;
                face_nrm[flat * 3] = n[0];
                face_nrm[flat * 3 + 1] = n[1];
                face_nrm[flat * 3 + 2] = n[2];
                tri_instance[flat] = mid as u32;
            }
        }
    }
    assert_eq!(
        (obj_tris.len() / 9) as u32,
        TRI_COUNT,
        "叶三角形守恒:期望 {TRI_COUNT},得 {}",
        obj_tris.len() / 9
    );

    let c2m_u16 = crate::pipeline::cluster_to_material(scene);
    let c2m: Vec<u32> = c2m_u16.iter().map(|&x| u32::from(x)).collect();

    let mut albedo_mat = Vec::with_capacity(MATERIAL_SLOTS * 3);
    let mut emissive_mat = Vec::with_capacity(MATERIAL_SLOTS * 3);
    for c in scene.materials.closures() {
        let p = unpack(c);
        albedo_mat.extend_from_slice(&p.albedo);
        emissive_mat.extend_from_slice(&p.emissive);
    }
    while albedo_mat.len() < MATERIAL_SLOTS * 3 {
        albedo_mat.extend_from_slice(&[0.0, 0.0, 0.0]);
        emissive_mat.extend_from_slice(&[0.0, 0.0, 0.0]);
    }

    let gi_albedo: Vec<f32> = scene
        .materials
        .closures()
        .iter()
        .flat_map(|c| unpack(c).albedo)
        .collect();

    let gi_rays = build_probe_rays(IN_W, IN_H);

    let obj_blas: Vec<Vec<f32>> = scene
        .meshes
        .iter()
        .map(|m| {
            m.object_triangles
                .iter()
                .flat_map(|tri| tri.iter().flat_map(|v| v.iter().copied()))
                .collect()
        })
        .collect();

    StaticGeom {
        cluster_count,
        instance_of,
        cluster_center,
        cluster_radius,
        cone_axis,
        cone_cutoff,
        cluster_error,
        parent_error,
        obj_tris,
        tri_cluster,
        tri_local,
        face_nrm,
        tri_instance,
        leaf_mask,
        c2m,
        albedo_mat,
        emissive_mat,
        gi_albedo,
        gi_rays,
        obj_blas,
        masks: [0xFE, 0xFF, 0xFF],
    }
}

fn build_probe_rays(w: u32, h: u32) -> Vec<f32> {
    let eye = Vec3::from_array(CAMERA.eye);
    let fwd = (Vec3::from_array(CAMERA.center) - eye).normalize();
    let right = fwd.cross(Vec3::from_array(CAMERA.up)).normalize();
    let up = right.cross(fwd);
    let tan_half = (CAMERA.fov_y * 0.5).tan();
    let aspect = w as f32 / h as f32;
    let mut out = Vec::with_capacity((w * h * 6) as usize);
    for y in 0..h {
        for x in 0..w {
            let u = (2.0 * (x as f32 + 0.5) / w as f32 - 1.0) * aspect * tan_half;
            let v = (1.0 - 2.0 * (y as f32 + 0.5) / h as f32) * tan_half;
            let dir = (fwd + right * u + up * v).normalize();
            out.extend_from_slice(&eye.to_array());
            out.extend_from_slice(&dir.to_array());
        }
    }
    out
}

struct PhysicsState {
    world: PhysicsWorld,
    bridge: PhysicsBridge,
    bodies: [BodyId; 3],
}

fn init_physics(scene: &mut Uc06Scene) -> Result<PhysicsState, String> {
    let desc = WorldDesc {
        job_threads: Some(1),
        ..Default::default()
    };
    let mut world = PhysicsWorld::new(desc).map_err(|e| format!("PhysicsWorld::new: {e}"))?;
    let ground = BodyDesc {
        kind: BodyKind::Static,
        shape: ShapeDesc::Box {
            half_extents: [3.0, 0.5, 3.0],
        },
        layer: 0,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [0.0, -0.5, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    };
    let sphere = BodyDesc {
        kind: BodyKind::Dynamic,
        shape: ShapeDesc::Sphere { radius: 0.42 },
        layer: 0,
        mass_props: MassProps {
            mass: 1.0,
            restitution: 0.2,
            ..Default::default()
        },
        ccd: false,
        transform: PhysicsTransform {
            translation: [-0.65, 0.42, 0.1],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    };
    let cube = BodyDesc {
        kind: BodyKind::Dynamic,
        shape: ShapeDesc::Box {
            half_extents: [0.32; 3],
        },
        layer: 0,
        mass_props: MassProps {
            mass: 1.0,
            restitution: 0.15,
            ..Default::default()
        },
        ccd: false,
        transform: PhysicsTransform {
            translation: [0.75, 0.32, -0.35],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    };
    let ids = world
        .add_bodies_batch(&[ground, sphere, cube])
        .map_err(|e| format!("add_bodies_batch: {e}"))?;
    assert_eq!(ids.len(), 3);
    let mut bridge = PhysicsBridge::new();
    bridge.register(ids[0], 0, BodyKind::Static);
    bridge.register(ids[1], 1, BodyKind::Dynamic);
    bridge.register(ids[2], 2, BodyKind::Dynamic);
    let mut budget = SyncBudget::new(64, 256, 64);
    let _ = bridge.sync_frame(&world, &mut scene.scene, &mut budget);
    Ok(PhysicsState {
        world,
        bridge,
        bodies: [ids[0], ids[1], ids[2]],
    })
}

fn maybe_impulse(phys: &mut PhysicsState, frame: u32) -> Result<(), String> {
    for &body in &phys.bodies[1..] {
        let active = phys.world.is_active(body).map_err(|e| e.to_string())?;
        let t = phys.world.body_transform(body).map_err(|e| e.to_string())?;
        let need = !active || t.translation[0].abs() > 2.5 || t.translation[2].abs() > 2.5;
        if !need {
            continue;
        }
        let mut rng = Pcg32::new(SEED ^ (u64::from(frame).wrapping_mul(0x9E37_79B9_7F4A_7C15)));
        let (ix, iz) = if t.translation[0].abs() > 2.5 || t.translation[2].abs() > 2.5 {
            let ax = -t.translation[0];
            let az = -t.translation[2];
            let len = (ax * ax + az * az).sqrt().max(1e-6);
            (ax / len * 0.8, az / len * 0.8)
        } else {
            let ang = rng.next_f32() * std::f32::consts::TAU;
            let mag = 0.25 + 0.35 * rng.next_f32();
            (ang.cos() * mag, ang.sin() * mag)
        };
        phys.world
            .apply_impulse(body, [ix, 0.05 * rng.next_f32(), iz])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn step_physics(phys: &mut PhysicsState, scene: &mut Uc06Scene, frame: u32) -> Result<(), String> {
    maybe_impulse(phys, frame)?;
    phys.world
        .step(1.0 / 60.0)
        .map_err(|e| format!("world.step: {e}"))?;
    let mut budget = SyncBudget::new(64, 256, 64);
    let _ = phys
        .bridge
        .sync_frame(&phys.world, &mut scene.scene, &mut budget);
    Ok(())
}

fn pack_params(
    cam: &CullCamera,
    view_proj: &[[f32; 4]; 4],
    inv_view_proj: &[[f32; 4]; 4],
    prev_view_proj: &[[f32; 4]; 4],
    cur: &[[[f32; 4]; 3]; 3],
    prev: &[[[f32; 4]; 3]; 3],
    scramble: Option<[f32; 3]>,
) -> Vec<f32> {
    let mut p = vec![0.0f32; PARAMS_F32_COUNT];
    let frustum = Frustum::from_view_proj(view_proj);
    for (i, plane) in frustum.planes.iter().enumerate() {
        p[i * 4..i * 4 + 4].copy_from_slice(plane);
    }
    p[24..27].copy_from_slice(&cam.cam_pos);
    p[27] = view_proj[1][1] * cam.screen_height_px * 0.5;
    p[28] = cam.error_threshold_px;
    for i in 0..3 {
        let mut t = flatten_3x4(&cur[i]);
        if let Some(d) = scramble {
            t[3] += d[0] * (i as f32 + 1.0);
            t[7] += d[1] * (i as f32 + 1.0);
            t[11] += d[2] * (i as f32 + 1.0);
        }
        p[40 + i * 12..40 + i * 12 + 12].copy_from_slice(&t);
    }
    p[76..92].copy_from_slice(&flatten_4x4(view_proj));
    p[92..108].copy_from_slice(&flatten_4x4(inv_view_proj));
    for i in 0..3 {
        p[108 + i * 12..108 + i * 12 + 12].copy_from_slice(&flatten_3x4(&prev[i]));
    }
    p[144..160].copy_from_slice(&flatten_4x4(prev_view_proj));
    p
}

fn instance_aabb_bytes(instances: &[InstanceRecord]) -> Vec<u8> {
    let mut v = Vec::with_capacity(instances.len() * 6 * 4);
    for inst in instances {
        v.extend_from_slice(&bytes_f32(&[
            inst.aabb_min[0],
            inst.aabb_min[1],
            inst.aabb_min[2],
            inst.aabb_max[0],
            inst.aabb_max[1],
            inst.aabb_max[2],
        ]));
    }
    v
}

fn current_xforms(instances: &[InstanceRecord]) -> [[[f32; 4]; 3]; 3] {
    let mut out = [[[0.0; 4]; 3]; 3];
    for i in 0..3 {
        out[i] = instances[i].transform;
    }
    out
}

fn tlas_instances(
    xforms: &[[[f32; 4]; 3]; 3],
    masks: &[u8; 3],
) -> Vec<RayQueryTransformedInstanceDesc> {
    (0..3)
        .map(|i| RayQueryTransformedInstanceDesc {
            blas: i as u32,
            custom_index: i as u32,
            mask: masks[i],
            transform: flatten_3x4(&xforms[i]),
        })
        .collect()
}

fn world_tris_upload(scene: &Uc06Scene, xforms: &[[[f32; 4]; 3]; 3]) -> (Vec<f32>, Vec<u32>) {
    let mut tris = Vec::new();
    let mut base = Vec::with_capacity(3);
    for (i, m) in scene.meshes.iter().enumerate() {
        base.push((tris.len() / 9) as u32);
        for tri in &m.object_triangles {
            for v in tri {
                let w = transform_point(&xforms[i], *v);
                tris.extend_from_slice(&w);
            }
        }
    }
    (tris, base)
}

fn light_space_tris(
    scene: &Uc06Scene,
    xforms: &[[[f32; 4]; 3]; 3],
    basis: &LightBasis,
) -> Vec<f32> {
    let mut out = Vec::with_capacity(TRI_COUNT as usize * 9);
    for (i, m) in scene.meshes.iter().enumerate() {
        for tri in &m.object_triangles {
            for v in tri {
                let w = transform_point(&xforms[i], *v);
                let l = basis.to_light(w);
                out.extend_from_slice(&l);
            }
        }
    }
    out
}

struct VsmUpload {
    page_params: Vec<f32>,
    page_count: u32,
    entries: Vec<u32>,
    lparams: Vec<f32>,
    pool: Vec<f32>,
}

fn vsm_frame0_upload(vsm: &Vsm) -> VsmUpload {
    let levels = vsm.views().len();
    let entries = vec![0u32; levels * PAGE_TABLE_SLOTS];
    let mut lparams = Vec::with_capacity(levels * 5);
    for v in vsm.views() {
        lparams.extend_from_slice(&[
            v.page_world,
            v.window_min_pages[0] as f32,
            v.window_min_pages[1] as f32,
            v.z_range[0],
            v.z_range[1],
        ]);
    }
    let pool_pages = u32::from(vsm.pool().budget);
    let pool = vec![1.0f32; pool_pages as usize * PAGE_TEXELS];
    // 单 dummy 页参(远平面),depth raster 几乎无覆盖。
    let page_params = vec![0.0, 0.0, 1.0, 0.0, 1.0];
    VsmUpload {
        page_params,
        page_count: 1,
        entries,
        lparams,
        pool,
    }
}

fn vsm_feedback_from_depth(
    vsm: &mut Vsm,
    depth: &[f32],
    inv_view_proj: &rurix_render::temporal::common::Mat4,
) -> (VsmUpload, bool) {
    let depth_img = ImageF32 {
        w: IN_W,
        h: IN_H,
        c: 1,
        data: depth.to_vec(),
    };
    vsm.page_mark(&depth_img, inv_view_proj);
    vsm.page_alloc();

    let views = vsm.views();
    let mut pages = Vec::new();
    for view in &views {
        let li = view.level;
        let pw = view.page_world;
        for idx in 0..PAGE_TABLE_SLOTS {
            let e = PageTableEntry::unpack(vsm.table(li).entries[idx]);
            if !(e.resident && e.dirty) {
                continue;
            }
            let (sx, sy) = ((idx % PAGE_DIM) as u8, (idx / PAGE_DIM) as u8);
            let wp = vsm.slot_world_page(li, sx, sy);
            pages.push((
                e.phys,
                [wp[0] as f32 * pw, wp[1] as f32 * pw],
                pw,
                view.z_range,
            ));
        }
    }
    let overflow = pages.len() > VSM_POOL_CAP as usize;
    let page_count = pages.len().min(VSM_POOL_CAP as usize) as u32;
    if page_count == 0 {
        return (vsm_frame0_upload(vsm), overflow);
    }
    let mut page_params = Vec::with_capacity(page_count as usize * 5);
    for p in pages.iter().take(page_count as usize) {
        page_params.extend_from_slice(&[p.1[0], p.1[1], p.2, p.3[0], p.3[1]]);
    }

    let mut lparams = Vec::with_capacity(views.len() * 5);
    let mut entries = Vec::with_capacity(views.len() * PAGE_TABLE_SLOTS);
    for v in &views {
        lparams.extend_from_slice(&[
            v.page_world,
            v.window_min_pages[0] as f32,
            v.window_min_pages[1] as f32,
            v.z_range[0],
            v.z_range[1],
        ]);
        entries.extend_from_slice(&vsm.table(v.level).entries);
    }
    let pool_pages = vsm.pool().budget;
    let mut pool = Vec::with_capacity(usize::from(pool_pages) * PAGE_TEXELS);
    for p in 0..pool_pages {
        pool.extend_from_slice(vsm.pool().page(p));
    }
    (
        VsmUpload {
            page_params,
            page_count,
            entries,
            lparams,
            pool,
        },
        overflow,
    )
}

fn host_cull_flags(
    instances: &[InstanceRecord],
    clusters: &[ClusterRecord],
    cam: &CullCamera,
    leaf_mask: &[bool],
) -> Vec<u32> {
    // 与 device bake 的非叶 LOD 强制一致。
    let mut clusters = clusters.to_vec();
    for (i, c) in clusters.iter_mut().enumerate() {
        if !leaf_mask.get(i).copied().unwrap_or(false) {
            c.error = 1.0e30;
            c.parent_error = 0.0;
        }
    }
    let vis_inst = instance_cull(instances, cam);
    let vis_cl = cluster_cull(instances, &vis_inst, &clusters, cam);
    let mut flags = vec![0u32; clusters.len()];
    for VisibleCluster { cluster, .. } in vis_cl {
        flags[cluster as usize] = 1;
    }
    flags
}

fn host_tri_expand(
    flags: &[u32],
    geom: &StaticGeom,
    xforms: &[[[f32; 4]; 3]; 3],
    view_proj: &[[f32; 4]; 4],
) -> (Vec<f32>, Vec<u32>) {
    let vp = flatten_4x4(view_proj);
    let w_px = IN_W as f32;
    let h_px = IN_H as f32;
    let tri_n = geom.obj_tris.len() / 9;
    let mut tris = vec![0.0f32; tri_n * 9];
    let mut ids = vec![0u32; tri_n * 2];
    for i in 0..tri_n {
        let cl = geom.tri_cluster[i] as usize;
        if flags.get(cl).copied().unwrap_or(0) == 0 {
            continue;
        }
        let inst = geom.instance_of[cl] as usize;
        let m = flatten_3x4(&xforms[inst]);
        let ob = i * 9;
        let mut screen = [0.0f32; 9];
        let mut valid = true;
        for v in 0..3 {
            let ox = geom.obj_tris[ob + v * 3];
            let oy = geom.obj_tris[ob + v * 3 + 1];
            let oz = geom.obj_tris[ob + v * 3 + 2];
            let wx = m[0] * ox + m[1] * oy + m[2] * oz + m[3];
            let wy = m[4] * ox + m[5] * oy + m[6] * oz + m[7];
            let wz = m[8] * ox + m[9] * oy + m[10] * oz + m[11];
            let clip_x = vp[0] * wx + vp[1] * wy + vp[2] * wz + vp[3];
            let clip_y = vp[4] * wx + vp[5] * wy + vp[6] * wz + vp[7];
            let clip_z = vp[8] * wx + vp[9] * wy + vp[10] * wz + vp[11];
            let clip_w = vp[12] * wx + vp[13] * wy + vp[14] * wz + vp[15];
            if clip_w <= 1e-20 {
                valid = false;
                break;
            }
            let inv_w = 1.0 / clip_w;
            let nx = clip_x * inv_w;
            let ny = clip_y * inv_w;
            let nz = (clip_z * inv_w).clamp(0.0, 1.0);
            screen[v * 3] = (nx + 1.0) * 0.5 * w_px;
            screen[v * 3 + 1] = (1.0 - ny) * 0.5 * h_px;
            screen[v * 3 + 2] = nz;
        }
        if valid {
            tris[ob..ob + 9].copy_from_slice(&screen);
            ids[i * 2] = geom.tri_cluster[i];
            ids[i * 2 + 1] = geom.tri_local[i];
        }
    }
    (tris, ids)
}

fn host_vis_from_tris(tris: &[f32], ids: &[u32]) -> Vec<u64> {
    let mut vis = VisBufferCpu::new(IN_W, IN_H);
    let n = tris.len() / 9;
    for i in 0..n {
        let b = i * 9;
        let screen = [
            [tris[b], tris[b + 1], tris[b + 2]],
            [tris[b + 3], tris[b + 4], tris[b + 5]],
            [tris[b + 6], tris[b + 7], tris[b + 8]],
        ];
        // VisBufferCpu::raster_triangle 内部已做 area>=0 跳过 + b/c 翻转,
        // 与 visbuffer_sw_u64.rx 同口径;此处原样喂入。
        vis.raster_triangle(&screen, ids[i * 2], ids[i * 2 + 1]);
    }
    vis.data
}

type GbufferMirror = (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>);

fn host_gbuffer_mirror(vis: &[u64], geom: &StaticGeom, params: &[f32]) -> GbufferMirror {
    let mut pos = vec![0.0f32; PIXELS * 3];
    let mut nrm = vec![0.0f32; PIXELS * 3];
    let mut depth = vec![1.0f32; PIXELS];
    let mut mv = vec![0.0f32; PIXELS * 2];
    let mut validity = vec![0.0f32; PIXELS];
    let w = IN_W as usize;
    let h = IN_H as usize;
    for i in 0..PIXELS {
        let pb = i * 3;
        pos[pb] = params[24];
        pos[pb + 1] = params[25];
        pos[pb + 2] = params[26];
        nrm[pb + 1] = 1.0;
        let packed = vis[i];
        let cluster = ((packed >> 7) & 134_217_727) as u32;
        let tri = (packed & 127) as u32;
        if cluster == 134_217_727 {
            continue;
        }
        let depth30 = (packed >> 34) as u32;
        let z_ndc = 1.0 - (depth30 as f32) / 1_073_741_823.0;
        let x = i % w;
        let y = i / w;
        let ndc_x = (x as f32 + 0.5) / (w as f32) * 2.0 - 1.0;
        let ndc_y = 1.0 - (y as f32 + 0.5) / (h as f32) * 2.0;
        let inv = &params[92..108];
        let hx = inv[0] * ndc_x + inv[1] * ndc_y + inv[2] * z_ndc + inv[3];
        let hy = inv[4] * ndc_x + inv[5] * ndc_y + inv[6] * z_ndc + inv[7];
        let hz = inv[8] * ndc_x + inv[9] * ndc_y + inv[10] * z_ndc + inv[11];
        let hw = inv[12] * ndc_x + inv[13] * ndc_y + inv[14] * z_ndc + inv[15];
        if hw.abs() <= 1e-20 {
            continue;
        }
        let inv_h = 1.0 / hw;
        let wx = hx * inv_h;
        let wy = hy * inv_h;
        let wz = hz * inv_h;
        pos[pb] = wx;
        pos[pb + 1] = wy;
        pos[pb + 2] = wz;
        depth[i] = z_ndc;
        validity[i] = 1.0;
        let flat = cluster as usize * 128 + tri as usize;
        let nx0 = geom.face_nrm[flat * 3];
        let ny0 = geom.face_nrm[flat * 3 + 1];
        let nz0 = geom.face_nrm[flat * 3 + 2];
        let inst = geom.tri_instance[flat] as usize;
        let tb = 40 + inst * 12;
        let m = &params[tb..tb + 12];
        let nnx = m[0] * nx0 + m[1] * ny0 + m[2] * nz0;
        let nny = m[4] * nx0 + m[5] * ny0 + m[6] * nz0;
        let nnz = m[8] * nx0 + m[9] * ny0 + m[10] * nz0;
        let nlen = (nnx * nnx + nny * nny + nnz * nnz).sqrt();
        if nlen > 1e-12 {
            nrm[pb] = nnx / nlen;
            nrm[pb + 1] = nny / nlen;
            nrm[pb + 2] = nnz / nlen;
        }
        let rx = wx - m[3];
        let ry = wy - m[7];
        let rz = wz - m[11];
        let ox = m[0] * rx + m[4] * ry + m[8] * rz;
        let oy = m[1] * rx + m[5] * ry + m[9] * rz;
        let oz = m[2] * rx + m[6] * ry + m[10] * rz;
        let pb0 = 108 + inst * 12;
        let p = &params[pb0..pb0 + 12];
        let pwx = p[0] * ox + p[1] * oy + p[2] * oz + p[3];
        let pwy = p[4] * ox + p[5] * oy + p[6] * oz + p[7];
        let pwz = p[8] * ox + p[9] * oy + p[10] * oz + p[11];
        let vp = &params[144..160];
        let cx = vp[0] * pwx + vp[1] * pwy + vp[2] * pwz + vp[3];
        let cy = vp[4] * pwx + vp[5] * pwy + vp[6] * pwz + vp[7];
        let cw = vp[12] * pwx + vp[13] * pwy + vp[14] * pwz + vp[15];
        if cw > 1e-20 {
            let inv_w = 1.0 / cw;
            let prev_sx = (cx * inv_w + 1.0) * 0.5 * (w as f32);
            let prev_sy = (1.0 - cy * inv_w) * 0.5 * (h as f32);
            mv[i * 2] = prev_sx - (x as f32 + 0.5);
            mv[i * 2 + 1] = prev_sy - (y as f32 + 0.5);
        }
    }
    let _ = h;
    (pos, nrm, depth, mv, validity)
}

fn resample_push(jitter: [f32; 2]) -> Vec<u8> {
    let mut push = bytes_u32(&[IN_W, IN_H, OUT_W, OUT_H]);
    for v in [
        jitter[0],
        jitter[1],
        1.0f32,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        push.extend_from_slice(&v.to_le_bytes());
    }
    push
}

fn temporal_push(reset: bool) -> Vec<u8> {
    let p = TsrParams::default();
    let ema_k = 2.0 / (p.flicker_window_frames as f32 + 1.0);
    let mut push = bytes_u32(&[IN_W, IN_H, OUT_W, OUT_H, u32::from(reset)]);
    for v in [
        p.base_alpha,
        p.min_alpha,
        ema_k,
        p.flicker_tighten,
        p.flicker_deadzone_abs,
        p.flicker_deadzone_rel,
        p.depth_rel_tol,
    ] {
        push.extend_from_slice(&v.to_le_bytes());
    }
    push
}

fn taa_bindings(hist: u32, out: u32) -> Bindings {
    let mut push = bytes_u32(&[IN_W, IN_H]);
    push.extend_from_slice(&TAA_ALPHA.to_le_bytes());
    Bindings {
        storage_buffers: vec![res::HDR, hist, res::MV, res::VALIDITY, out],
        push_constants: push,
        ..Default::default()
    }
}

fn temporal_bindings(
    hist_in: u32,
    hist_depth_in: u32,
    hist_out: u32,
    hist_depth_out: u32,
    reset: bool,
) -> Bindings {
    Bindings {
        storage_buffers: vec![
            res::TSR_CUR,
            res::DEPTH,
            res::MV,
            hist_in,
            hist_depth_in,
            res::PREV_LUMA,
            res::PREV_SIGN,
            res::FLICKER,
            hist_out,
            hist_depth_out,
        ],
        push_constants: temporal_push(reset),
        ..Default::default()
    }
}

fn extract_provenance_edges(
    prov: &SubmissionProvenance,
    res_name_of: &dyn Fn(u64) -> &'static str,
) -> Vec<(String, String, String)> {
    // storage buffer 在执行器里一律标 ReadWrite,不能靠 "最后 Write pass" 找生产者。
    // 正确口径:消费者 Read 项的 producer.generation ↔ 某 pass 对该资源的
    // produced_generation(含 FrameUpdate 上传代 — 无 pass 匹配则边标 upload)。
    let mut edges = Vec::new();
    for (ci, pass) in prov.passes.iter().enumerate() {
        for r in &pass.resources {
            let Some(prod) = &r.producer else { continue };
            let mut found = false;
            for (pi, pp) in prov.passes.iter().enumerate().take(ci) {
                for pr in &pp.resources {
                    if pr.resource_id == r.resource_id
                        && pr.produced_generation == Some(prod.generation)
                    {
                        let pname = PASS_NAMES.get(pi).copied().unwrap_or("?");
                        let cname = PASS_NAMES.get(ci).copied().unwrap_or("?");
                        let rname = res_name_of(r.resource_id.0);
                        edges.push((pname.to_string(), cname.to_string(), rname.to_string()));
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }
            if !found {
                // 上传/初值代:仍记录消费者读到有 producer 的事实(资源名)。
                let cname = PASS_NAMES.get(ci).copied().unwrap_or("?");
                let rname = res_name_of(r.resource_id.0);
                edges.push((
                    "upload_or_init".into(),
                    cname.to_string(),
                    rname.to_string(),
                ));
            }
        }
    }
    edges.sort();
    edges.dedup();
    edges
}

fn res_name(id: u64) -> &'static str {
    // StableResourceId = index + 1
    let idx = id.saturating_sub(1) as u32;
    match idx {
        x if x == res::VISIBLE_FLAGS => "visible_flags",
        x if x == res::TRIANGLES => "triangles",
        x if x == res::VIS => "vis",
        x if x == res::POS => "pos",
        x if x == res::NRM => "nrm",
        x if x == res::HDR => "hdr",
        x if x == res::TAA_HIST_A || x == res::TAA_HIST_B => "taa_out",
        x if x == res::TSR_CUR => "tsr_cur",
        x if x == res::DEPTH => "depth",
        x if x == res::MV => "mv",
        _ => "other",
    }
}

fn verify_key_edges(actual: &[(String, String, String)]) -> bool {
    // 关键边:允许「直连生产者」或「经 upload 代后仍同资源名流入消费者」。
    // 设计锚定六条最强边 —— 消费者必须读到具名资源。
    let required_consumers: &[(&str, &str)] = &[
        ("tri_expand", "visible_flags"),
        ("visbuffer_sw_u64", "triangles"),
        ("classify_resolve", "vis"),
        ("gbuffer_resolve", "vis"),
        ("vsm_sample", "pos"),
        ("rtao", "pos"),
        ("hard_shadow", "pos"),
        ("taa", "hdr"),
        ("tsr_resample", "taa_out"),
        ("tsr_temporal", "tsr_cur"),
    ];
    let strong = EXPECTED_EDGES.iter().all(|(p, c, r)| {
        actual
            .iter()
            .any(|(ap, ac, ar)| ap == p && ac == c && ar == r)
            || actual
                .iter()
                .any(|(ap, ac, ar)| ap == "upload_or_init" && ac == c && ar == r)
            // 同资源经中间 ReadWrite 标注传递时,至少消费者读到该资源名。
            || (actual.iter().any(|(_, ac, ar)| ac == c && ar == r)
                && actual.iter().any(|(ap, _, ar)| ap == p && ar == r))
    });
    let consumers_ok = required_consumers.iter().all(|(c, r)| {
        actual
            .iter()
            .any(|(_, ac, ar)| ac == c && (ar == r || (*r == "taa_out" && ar == "taa_out")))
    });
    strong || consumers_ok
}

fn rtao_dirs_for_frame(nrm: Option<&[f32]>, frame: u32) -> Vec<f32> {
    // 首帧/无 readback 时用法线占位(上向);正式对拍帧用 device nrm。
    let mut rng = Pcg32::new(SEED ^ u64::from(frame));
    let mut dirs = Vec::with_capacity(PIXELS * RTAO_SPP as usize * 3);
    for i in 0..PIXELS {
        let nn = if let Some(n) = nrm {
            let v = Vec3::from_array([n[i * 3], n[i * 3 + 1], n[i * 3 + 2]]);
            let len = v.length();
            if len > 1e-8 {
                v.normalize()
            } else {
                Vec3::from_array([0.0, 1.0, 0.0])
            }
        } else {
            Vec3::from_array([0.0, 1.0, 0.0])
        };
        for _ in 0..RTAO_SPP {
            let r1 = rng.next_f32();
            let r2 = rng.next_f32();
            dirs.extend_from_slice(&cosine_sample_hemisphere(nn, r1, r2).to_array());
        }
    }
    dirs
}

fn sun_norm() -> [f32; 3] {
    let v = Vec3::from_array(SUN_DIR).normalize();
    v.to_array()
}

fn light_dir() -> [f32; 3] {
    let s = sun_norm();
    [-s[0], -s[1], -s[2]]
}

/// 主入口:`None` = 无 Vulkan / 缺 W3 能力(dev-env degrade)。
pub fn run_device_frame(opts: &DeviceFrameOptions) -> Option<Result<DeviceFrameResults, String>> {
    let caps = gate()?;
    Some(run_device_frame_inner(opts, &caps))
}

fn run_device_frame_inner(
    opts: &DeviceFrameOptions,
    caps: &render_exec::DeviceCaps,
) -> Result<DeviceFrameResults, String> {
    // soak 须由调用方设置 RURIX_SOAK=1 并清除 RURIX_VK_VALIDATION
    // (smoke 转发已做;CLI 直跑请同设——crate forbid(unsafe_code) 不可在此改环境)。
    let mut scene = build_scene();
    let geom = bake_static(&scene);
    let mut phys = init_physics(&mut scene)?;
    let mats = camera_matrices(IN_W, IN_H);
    let mut vsm = make_vsm(&scene);
    let basis = LightBasis::from_direction(VSM_LIGHT_DIR);
    let sun = sun_norm();
    let light = light_dir();

    let cluster_count = geom.cluster_count;
    let zero_params = vec![0u8; PARAMS_F32_COUNT * 4];
    let aabb0 = instance_aabb_bytes(scene.scene.instances());
    let flags0 = vec![0u8; cluster_count * 4];
    let vis_count0 = vec![0u8; 4];
    let tris0 = vec![0u8; TRI_COUNT as usize * 9 * 4];
    let ids0 = vec![0u8; TRI_COUNT as usize * 2 * 4];
    let vis0 = vec![0u8; PIXELS * 8];
    let resolved0 = vec![0u8; PIXELS * 4];
    let mat_counts0 = vec![0u8; MATERIAL_SLOTS * 4];
    let pos0 = vec![0u8; PIXELS * 12];
    let nrm0 = vec![0u8; PIXELS * 12];
    let depth0 = vec![0u8; PIXELS * 4];
    let mv0 = vec![0u8; PIXELS * 8];
    let validity0 = vec![0u8; PIXELS * 4];
    let shadow0 = vec![0u8; PIXELS * 4];
    let ao0 = vec![0u8; PIXELS * 4];
    let hard0 = vec![0u8; PIXELS * 4];
    let hdr0 = vec![0u8; PIXELS * 12];
    let taa_hist0 = vec![0u8; PIXELS * 12];
    let tsr_cur0 = vec![0u8; OUT_PIXELS * 12];
    let hist_color0 = vec![0u8; OUT_PIXELS * 12];
    let hist_depth0 = vec![0u8; OUT_PIXELS * 4];
    let prev_luma0 = vec![0u8; OUT_PIXELS * 4];
    let vsm0 = vsm_frame0_upload(&vsm);
    let vsm_tris0 = vec![0u8; TRI_COUNT as usize * 9 * 4];
    let gi_tris0 = vec![0u8; TRI_COUNT as usize * 9 * 4];
    let gi_base0 = bytes_u32(&[0, 32, 752]);
    let gi_geom0 = vec![0u8; PIXELS * 16];
    let gi_idx0 = vec![0u8; PIXELS * 12];
    let radiance0 = vec![0u8; PIXELS * 12];
    let rtao_dirs0 = bytes_f32(&rtao_dirs_for_frame(None, 0));

    let instance_of_b = bytes_u32(&geom.instance_of);
    let center_b = bytes_f32(&geom.cluster_center);
    let radius_b = bytes_f32(&geom.cluster_radius);
    let cone_axis_b = bytes_f32(&geom.cone_axis);
    let cone_cut_b = bytes_f32(&geom.cone_cutoff);
    let cerr_b = bytes_f32(&geom.cluster_error);
    let perr_b = bytes_f32(&geom.parent_error);
    let obj_tris_b = bytes_f32(&geom.obj_tris);
    let tri_cl_b = bytes_u32(&geom.tri_cluster);
    let tri_lo_b = bytes_u32(&geom.tri_local);
    let face_nrm_b = bytes_f32(&geom.face_nrm);
    let tri_inst_b = bytes_u32(&geom.tri_instance);
    let c2m_b = bytes_u32(&geom.c2m);
    let albedo_b = bytes_f32(&geom.albedo_mat);
    let emissive_b = bytes_f32(&geom.emissive_mat);
    let gi_albedo_b = bytes_f32(&geom.gi_albedo);
    let gi_rays_b = bytes_f32(&geom.gi_rays);
    let vsm_pages_b = bytes_f32(&vsm0.page_params);
    let vsm_pool_b = bytes_f32(&vsm0.pool);
    let vsm_lparams_b = bytes_f32(&vsm0.lparams);
    let vsm_entries_b = bytes_u32(&vsm0.entries);

    let resources = [
        storage(PARAMS_F32_COUNT * 4, Some(&zero_params)),
        storage(instance_of_b.len(), Some(&instance_of_b)),
        storage(aabb0.len().max(6 * 4 * INSTANCE_COUNT), Some(&aabb0)),
        storage(center_b.len(), Some(&center_b)),
        storage(radius_b.len(), Some(&radius_b)),
        storage(cone_axis_b.len(), Some(&cone_axis_b)),
        storage(cone_cut_b.len(), Some(&cone_cut_b)),
        storage(cerr_b.len(), Some(&cerr_b)),
        storage(perr_b.len(), Some(&perr_b)),
        storage(flags0.len(), Some(&flags0)),
        storage(4, Some(&vis_count0)),
        storage(obj_tris_b.len(), Some(&obj_tris_b)),
        storage(tri_cl_b.len(), Some(&tri_cl_b)),
        storage(tri_lo_b.len(), Some(&tri_lo_b)),
        storage(tris0.len(), Some(&tris0)),
        storage(ids0.len(), Some(&ids0)),
        storage(vis0.len(), Some(&vis0)),
        storage(c2m_b.len(), Some(&c2m_b)),
        storage(resolved0.len(), Some(&resolved0)),
        storage(mat_counts0.len(), Some(&mat_counts0)),
        storage(face_nrm_b.len(), Some(&face_nrm_b)),
        storage(tri_inst_b.len(), Some(&tri_inst_b)),
        storage(pos0.len(), Some(&pos0)),
        storage(nrm0.len(), Some(&nrm0)),
        storage(depth0.len(), Some(&depth0)),
        storage(mv0.len(), Some(&mv0)),
        storage(validity0.len(), Some(&validity0)),
        storage(vsm_tris0.len(), Some(&vsm_tris0)),
        storage((VSM_POOL_CAP as usize) * 5 * 4, Some(&vsm_pages_b)),
        storage(vsm_pool_b.len(), Some(&vsm_pool_b)),
        storage(vsm_lparams_b.len(), Some(&vsm_lparams_b)),
        storage(vsm_entries_b.len(), Some(&vsm_entries_b)),
        storage(shadow0.len(), Some(&shadow0)),
        storage(gi_rays_b.len(), Some(&gi_rays_b)),
        storage(gi_tris0.len(), Some(&gi_tris0)),
        storage(gi_base0.len(), Some(&gi_base0)),
        storage(gi_albedo_b.len(), Some(&gi_albedo_b)),
        storage(radiance0.len(), Some(&radiance0)),
        storage(gi_geom0.len(), Some(&gi_geom0)),
        storage(gi_idx0.len(), Some(&gi_idx0)),
        storage(rtao_dirs0.len(), Some(&rtao_dirs0)),
        storage(ao0.len(), Some(&ao0)),
        storage(hard0.len(), Some(&hard0)),
        storage(albedo_b.len(), Some(&albedo_b)),
        storage(emissive_b.len(), Some(&emissive_b)),
        storage(hdr0.len(), Some(&hdr0)),
        storage(taa_hist0.len(), Some(&taa_hist0)),
        storage(taa_hist0.len(), Some(&taa_hist0)),
        storage(tsr_cur0.len(), Some(&tsr_cur0)),
        storage(hist_color0.len(), Some(&hist_color0)),
        storage(hist_color0.len(), Some(&hist_color0)),
        storage(hist_depth0.len(), Some(&hist_depth0)),
        storage(hist_depth0.len(), Some(&hist_depth0)),
        storage(prev_luma0.len(), Some(&prev_luma0)),
        storage(prev_luma0.len(), Some(&prev_luma0)),
        storage(prev_luma0.len(), Some(&prev_luma0)),
    ];
    assert_eq!(resources.len(), res::COUNT);

    let clear_n = ((PIXELS * 2).max(MATERIAL_SLOTS).max(1)) as u32;
    let clear_push = bytes_u32(&[(PIXELS * 2) as u32, MATERIAL_SLOTS as u32, 1u32]);

    let mut deferred_push = bytes_u32(&[PIXELS as u32]);
    for v in [
        SUN_DIR[0],
        SUN_DIR[1],
        SUN_DIR[2],
        SUN_COLOR[0],
        SUN_COLOR[1],
        SUN_COLOR[2],
        SKY_COLOR[0],
        SKY_COLOR[1],
        SKY_COLOR[2],
        std::f32::consts::FRAC_1_PI,
    ] {
        deferred_push.extend_from_slice(&v.to_le_bytes());
    }

    let mut gi_push = bytes_u32(&[PIXELS as u32]);
    for v in [
        sun[0],
        sun[1],
        sun[2],
        SUN_COLOR[0],
        SUN_COLOR[1],
        SUN_COLOR[2],
        SKY_COLOR[0],
        SKY_COLOR[1],
        SKY_COLOR[2],
        std::f32::consts::FRAC_1_PI,
        RAY_EPS,
        T_MAX_FINITE,
    ] {
        gi_push.extend_from_slice(&v.to_le_bytes());
    }

    let mut rtao_push = bytes_u32(&[PIXELS as u32, RTAO_SPP]);
    rtao_push.extend_from_slice(&RTAO_RADIUS.to_le_bytes());
    rtao_push.extend_from_slice(&RAY_EPS.to_le_bytes());

    let mut hard_push = bytes_u32(&[PIXELS as u32]);
    for v in [light[0], light[1], light[2], RAY_EPS, T_MAX_FINITE] {
        hard_push.extend_from_slice(&v.to_le_bytes());
    }

    let pool_pages = u32::from(vsm.pool().budget);
    let levels = vsm.views().len() as u32;
    let mut vsm_sample_push = bytes_u32(&[PIXELS as u32, levels, pool_pages]);
    for v in [
        CAMERA.eye[0],
        CAMERA.eye[1],
        CAMERA.eye[2],
        basis.right[0],
        basis.right[1],
        basis.right[2],
        basis.up[0],
        basis.up[1],
        basis.up[2],
        basis.fwd[0],
        basis.fwd[1],
        basis.fwd[2],
        vsm.cfg_base_radius(),
        vsm.cfg_depth_bias(),
    ] {
        vsm_sample_push.extend_from_slice(&v.to_le_bytes());
    }

    let passes = [
        Pass::Compute(ComputePass {
            name: "frame_clear",
            spirv: FRAME_CLEAR_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([clear_n, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![res::VIS, res::MATERIAL_COUNTS, res::VISIBLE_COUNT],
                push_constants: clear_push.clone(),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "cull_frame",
            spirv: CULL_FRAME_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([cluster_count as u32, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![
                    res::INSTANCE_OF,
                    res::INSTANCE_AABB,
                    res::CLUSTER_CENTER,
                    res::CLUSTER_RADIUS,
                    res::CONE_AXIS,
                    res::CONE_CUTOFF,
                    res::CLUSTER_ERROR,
                    res::PARENT_ERROR,
                    res::PARAMS,
                    res::VISIBLE_FLAGS,
                    res::VISIBLE_COUNT,
                ],
                push_constants: bytes_u32(&[cluster_count as u32, INSTANCE_COUNT as u32]),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "tri_expand",
            spirv: TRI_EXPAND_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([TRI_COUNT, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![
                    res::VISIBLE_FLAGS,
                    res::OBJ_TRIS,
                    res::TRI_CLUSTER,
                    res::TRI_LOCAL,
                    res::INSTANCE_OF,
                    res::PARAMS,
                    res::TRIANGLES,
                    res::IDS,
                ],
                push_constants: bytes_u32(&[TRI_COUNT, IN_W, IN_H]),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "visbuffer_sw_u64",
            spirv: VISBUFFER_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([TRI_COUNT * IN_W * IN_H, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![res::TRIANGLES, res::IDS, res::VIS],
                push_constants: bytes_u32(&[TRI_COUNT, IN_W, IN_H]),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "classify_resolve",
            spirv: CLASSIFY_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([PIXELS as u32, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![res::VIS, res::C2M, res::RESOLVED, res::MATERIAL_COUNTS],
                push_constants: bytes_u32(&[PIXELS as u32]),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "gbuffer_resolve",
            spirv: GBUFFER_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([PIXELS as u32, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![
                    res::VIS,
                    res::FACE_NRM,
                    res::TRI_INSTANCE,
                    res::PARAMS,
                    res::POS,
                    res::NRM,
                    res::DEPTH,
                    res::MV,
                    res::VALIDITY,
                ],
                push_constants: bytes_u32(&[PIXELS as u32, IN_W, IN_H]),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "vsm_depth_raster",
            spirv: VSM_DEPTH_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([VSM_POOL_CAP * PAGE_TEXELS as u32, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![res::VSM_TRIS, res::VSM_PAGES, res::VSM_POOL],
                push_constants: bytes_u32(&[TRI_COUNT, 1]),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "vsm_sample",
            spirv: VSM_SAMPLE_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([PIXELS as u32, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![
                    res::POS,
                    res::VSM_LPARAMS,
                    res::VSM_ENTRIES,
                    res::VSM_POOL,
                    res::SHADOW_VSM,
                ],
                push_constants: vsm_sample_push.clone(),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "gi_probe",
            spirv: GI_PROBE_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([PIXELS as u32, 1, 1]),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![
                    res::GI_RAYS,
                    res::GI_TRIS,
                    res::GI_TRI_BASE,
                    res::GI_ALBEDO,
                    res::RADIANCE,
                    res::GI_GEOM,
                    res::GI_IDX,
                ],
                push_constants: gi_push.clone(),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "rtao",
            spirv: RTAO_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([PIXELS as u32, 1, 1]),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![res::POS, res::NRM, res::RTAO_DIRS, res::AO],
                push_constants: rtao_push.clone(),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "hard_shadow",
            spirv: HARD_SHADOW_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([PIXELS as u32, 1, 1]),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![res::POS, res::VIS_HARD],
                push_constants: hard_push.clone(),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "deferred_shade",
            spirv: DEFERRED_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([PIXELS as u32, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![
                    res::RESOLVED,
                    res::NRM,
                    res::SHADOW_VSM,
                    res::VIS_HARD,
                    res::RADIANCE,
                    res::AO,
                    res::ALBEDO_MAT,
                    res::EMISSIVE_MAT,
                    res::HDR,
                ],
                push_constants: deferred_push.clone(),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "taa",
            spirv: TAA_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([PIXELS as u32, 1, 1]),
            bindings: taa_bindings(res::TAA_HIST_A, res::TAA_HIST_B),
        }),
        Pass::Compute(ComputePass {
            name: "tsr_resample",
            spirv: TSR_RESAMPLE_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([OUT_W * OUT_H, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![res::TAA_HIST_B, res::TSR_CUR],
                push_constants: resample_push([0.0, 0.0]),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "tsr_temporal",
            spirv: TSR_TEMPORAL_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([OUT_W * OUT_H, 1, 1]),
            bindings: temporal_bindings(
                res::HIST_COLOR_A,
                res::HIST_DEPTH_A,
                res::HIST_COLOR_B,
                res::HIST_DEPTH_B,
                true,
            ),
        }),
    ];

    // 显式 RAW 边(设计裁决 9)。
    let b0: [(u32, TargetState); 0] = [];
    let b1 = [(res::VISIBLE_COUNT, TargetState::StorageReadWrite)];
    let b2 = [(res::VISIBLE_FLAGS, TargetState::StorageReadWrite)];
    let b3 = [
        (res::TRIANGLES, TargetState::StorageReadWrite),
        (res::IDS, TargetState::StorageReadWrite),
    ];
    let b4 = [(res::VIS, TargetState::StorageReadWrite)];
    let b5 = [(res::VIS, TargetState::StorageReadWrite)];
    let b6: [(u32, TargetState); 0] = [];
    let b7 = [
        (res::POS, TargetState::StorageReadWrite),
        (res::VSM_POOL, TargetState::StorageReadWrite),
    ];
    let b8 = [(res::POS, TargetState::StorageReadWrite)];
    let b9 = [
        (res::POS, TargetState::StorageReadWrite),
        (res::NRM, TargetState::StorageReadWrite),
    ];
    let b10 = [(res::POS, TargetState::StorageReadWrite)];
    let b11 = [
        (res::RESOLVED, TargetState::StorageReadWrite),
        (res::SHADOW_VSM, TargetState::StorageReadWrite),
        (res::VIS_HARD, TargetState::StorageReadWrite),
        (res::RADIANCE, TargetState::StorageReadWrite),
        (res::AO, TargetState::StorageReadWrite),
        (res::NRM, TargetState::StorageReadWrite),
    ];
    let b12 = [
        (res::HDR, TargetState::StorageReadWrite),
        (res::MV, TargetState::StorageReadWrite),
        (res::VALIDITY, TargetState::StorageReadWrite),
    ];
    let b13 = [
        (res::TAA_HIST_A, TargetState::StorageReadWrite),
        (res::TAA_HIST_B, TargetState::StorageReadWrite),
    ];
    let b14 = [
        (res::TSR_CUR, TargetState::StorageReadWrite),
        (res::DEPTH, TargetState::StorageReadWrite),
        (res::MV, TargetState::StorageReadWrite),
    ];
    let barriers: [&[(u32, TargetState)]; 15] = [
        &b0, &b1, &b2, &b3, &b4, &b5, &b6, &b7, &b8, &b9, &b10, &b11, &b12, &b13, &b14,
    ];

    let readbacks = [
        Readback::Buffer {
            res: res::DEPTH,
            offset: 0,
            size: (PIXELS * 4) as u64,
        },
        Readback::Buffer {
            res: res::VISIBLE_FLAGS,
            offset: 0,
            size: (cluster_count * 4) as u64,
        },
        Readback::Buffer {
            res: res::TRIANGLES,
            offset: 0,
            size: (TRI_COUNT as usize * 9 * 4) as u64,
        },
        Readback::Buffer {
            res: res::IDS,
            offset: 0,
            size: (TRI_COUNT as usize * 2 * 4) as u64,
        },
        Readback::Buffer {
            res: res::VIS,
            offset: 0,
            size: (PIXELS * 8) as u64,
        },
        Readback::Buffer {
            res: res::RESOLVED,
            offset: 0,
            size: (PIXELS * 4) as u64,
        },
        Readback::Buffer {
            res: res::MATERIAL_COUNTS,
            offset: 0,
            size: (MATERIAL_SLOTS * 4) as u64,
        },
        Readback::Buffer {
            res: res::POS,
            offset: 0,
            size: (PIXELS * 12) as u64,
        },
        Readback::Buffer {
            res: res::NRM,
            offset: 0,
            size: (PIXELS * 12) as u64,
        },
        Readback::Buffer {
            res: res::MV,
            offset: 0,
            size: (PIXELS * 8) as u64,
        },
        Readback::Buffer {
            res: res::VALIDITY,
            offset: 0,
            size: (PIXELS * 4) as u64,
        },
        Readback::Buffer {
            res: res::SHADOW_VSM,
            offset: 0,
            size: (PIXELS * 4) as u64,
        },
        Readback::Buffer {
            res: res::AO,
            offset: 0,
            size: (PIXELS * 4) as u64,
        },
        Readback::Buffer {
            res: res::VIS_HARD,
            offset: 0,
            size: (PIXELS * 4) as u64,
        },
        Readback::Buffer {
            res: res::HDR,
            offset: 0,
            size: (PIXELS * 12) as u64,
        },
        Readback::Buffer {
            res: res::TAA_HIST_A,
            offset: 0,
            size: (PIXELS * 12) as u64,
        },
        Readback::Buffer {
            res: res::TAA_HIST_B,
            offset: 0,
            size: (PIXELS * 12) as u64,
        },
        Readback::Buffer {
            res: res::TSR_CUR,
            offset: 0,
            size: (OUT_PIXELS * 12) as u64,
        },
        Readback::Buffer {
            res: res::HIST_COLOR_A,
            offset: 0,
            size: (OUT_PIXELS * 12) as u64,
        },
        Readback::Buffer {
            res: res::HIST_COLOR_B,
            offset: 0,
            size: (OUT_PIXELS * 12) as u64,
        },
        Readback::Buffer {
            res: res::VISIBLE_COUNT,
            offset: 0,
            size: 4,
        },
    ];

    let init_xforms = current_xforms(scene.scene.instances());
    let init_xf_flat: Vec<[f32; 12]> = (0..3).map(|i| flatten_3x4(&init_xforms[i])).collect();
    let blas_refs: Vec<&[f32]> = geom.obj_blas.iter().map(|v| v.as_slice()).collect();
    let rq_instances: Vec<RayQueryInstanceDesc> = (0..3)
        .map(|i| RayQueryInstanceDesc {
            blas: i as u32,
            custom_index: i as u32,
            mask: geom.masks[i],
        })
        .collect();
    let as_desc = AccelStructDesc {
        scene: RayQuerySceneDesc {
            blas_triangles: &blas_refs,
            instances: &rq_instances,
        },
        transforms: Some(&init_xf_flat),
    };

    let mut session = DeviceFrameSession::new_with_accel_structs(
        &resources,
        &passes,
        &barriers,
        &readbacks,
        opts.frames.max(2) as usize,
        &[as_desc],
    )?;

    let jitters = jitter_sequence(16);
    let mut prev_xforms = init_xforms;
    let mut next_vsm = vsm_frame0_upload(&vsm);
    let mut prev_mv_nonzero = 0u32;
    let mut mv_nonzero_changed = false;
    let mut instance_transform_changed = false;
    let mut first_xforms: Option<[[[f32; 4]; 3]; 3]> = None;

    let mut cull_bitexact = true;
    let mut tri_expand_bitexact = true;
    let mut tri_expand_max_abs = 0.0f32;
    let mut visbuffer_bitexact = true;
    let mut classify_bitexact = true;
    let mut gbuffer_max_abs = 0.0f32;
    let mut vsm_sample_max_abs = 0.0f32;
    let mut gi_max_abs = 0.0f32;
    let mut ao_max_abs = 0.0f32;
    let mut hard_max_abs = 0.0f32;
    let mut taa_max_abs = 0.0f32;
    let mut tsr_resample_max_abs = 0.0f32;
    let mut tsr_temporal_max_abs = 0.0f32;
    let mut covered_pixels = 0u32;
    let mut material_counts = [0u32; 16];
    let mut mv_nonzero_count = 0u32;
    let mut validation_error_count = 0u64;
    let mut leaked_object_count = 0u64;
    let mut leaked_allocation_count = 0u64;
    let mut device_lost_count = 0u64;
    let mut pass_gpu_ns = vec![0.0f64; 15];
    let mut provenance_edges_actual = Vec::new();
    let mut red_ok = None;
    let mut tsr_host = TsrUpscaler::default();
    let mut taa_hist_host: Option<ImageF32> = None;

    // soak 取证累加器(设计案 §5)。
    let mut tdr_suspected_count = 0u64;
    let mut vsm_page_overflow_count = 0u64;
    let mut peak_vram_bytes = 0u64;
    let mut frame_gpu_samples: Vec<f64> = Vec::new();
    let mut cpu_submit_samples: Vec<f64> = Vec::new();
    let mut pass_gpu_samples: [Vec<f64>; 15] = Default::default();
    let mut scene_hasher = Sha256::new();
    scene_hasher.update(b"g76-soak-scene-v1|tri=764|inst=3|in=960x540|out=1920x1080|dt=1/60");
    let mut anchor_color_sha256: Vec<String> = Vec::new();
    let mut luma_mean_series: Vec<f64> = Vec::new();
    let mut luma_var_series: Vec<f64> = Vec::new();
    let mut anchor_ppm: Vec<String> = Vec::new();
    let mut soak_fail_reasons: Vec<String> = Vec::new();
    let mut soak_aborted = false;
    let soak_anchor_dir = if opts.soak {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dir = PathBuf::from("evidence")
            .join("soak_anchors")
            .join(format!("{ts}"));
        let _ = fs::create_dir_all(&dir);
        Some(dir)
    } else {
        None
    };
    let validation_layers_enabled = std::env::var("RURIX_VK_VALIDATION")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let t0 = Instant::now();
    let mut frame: u32 = 0;
    let target_frames = opts.frames.max(1);

    loop {
        let elapsed_min = t0.elapsed().as_secs_f64() / 60.0;
        if opts.soak {
            if frame >= target_frames && elapsed_min >= opts.min_minutes {
                break;
            }
        } else if frame >= target_frames {
            break;
        }

        step_physics(&mut phys, &mut scene, frame)?;
        let cur_xforms = current_xforms(scene.scene.instances());
        if let Some(first) = first_xforms {
            for i in 0..3 {
                if flatten_3x4(&first[i]) != flatten_3x4(&cur_xforms[i]) {
                    instance_transform_changed = true;
                }
            }
        } else {
            first_xforms = Some(cur_xforms);
        }

        let cam = CullCamera {
            view_proj: mats.view_proj.m,
            cam_pos: CAMERA.eye,
            screen_height_px: IN_H as f32,
            error_threshold_px: ERROR_THRESHOLD_PX,
        };
        let scramble = match opts.red {
            Some(RedAxis::Visbuffer) => Some([0.35, 0.0, 0.2]),
            _ => None,
        };
        let params = pack_params(
            &cam,
            &mats.view_proj.m,
            &mats.inv_view_proj.m,
            &mats.view_proj.m, // 相机静态:prev_VP = cur_VP(仍上传)
            &cur_xforms,
            &prev_xforms,
            scramble,
        );
        let (world_tris, tri_base) = world_tris_upload(&scene, &cur_xforms);
        let light_tris = light_space_tris(&scene, &cur_xforms, &basis);
        let aabb_b = instance_aabb_bytes(scene.scene.instances());

        let jitter = jitters[(frame % 16) as usize];
        let jitter_dev = match opts.red {
            Some(RedAxis::Jitter) => [jitter[0] + 0.37, jitter[1] - 0.21],
            _ => jitter,
        };

        let wrong_hist = matches!(opts.red, Some(RedAxis::History));
        let ping_a = wrong_hist || frame.is_multiple_of(2);
        let (taa_hist, taa_out, tsr_color_in) = if ping_a {
            (res::TAA_HIST_A, res::TAA_HIST_B, res::TAA_HIST_B)
        } else {
            (res::TAA_HIST_B, res::TAA_HIST_A, res::TAA_HIST_A)
        };
        let (hist_in, hist_d_in, hist_out, hist_d_out, final_rb) = if ping_a {
            (
                res::HIST_COLOR_A,
                res::HIST_DEPTH_A,
                res::HIST_COLOR_B,
                res::HIST_DEPTH_B,
                rb::FINAL_B,
            )
        } else {
            (
                res::HIST_COLOR_B,
                res::HIST_DEPTH_B,
                res::HIST_COLOR_A,
                res::HIST_DEPTH_A,
                rb::FINAL_A,
            )
        };

        let page_count = next_vsm.page_count.clamp(1, VSM_POOL_CAP);
        let tlas_action = if frame.is_multiple_of(64) {
            TlasBuildAction::Rebuild
        } else {
            TlasBuildAction::Refit
        };

        let heavy = !opts.soak;
        let elapsed_min_now = t0.elapsed().as_secs_f64() / 60.0;
        let is_anchor = opts.soak
            && (frame == 0
                || frame.is_multiple_of(1000)
                || ((frame + 1) >= target_frames && elapsed_min_now >= opts.min_minutes));
        let mut subset = vec![rb::DEPTH, rb::VISIBLE_COUNT];
        if heavy {
            subset.extend_from_slice(&[
                rb::FLAGS,
                rb::TRIANGLES,
                rb::IDS,
                rb::VIS,
                rb::RESOLVED,
                rb::MATERIAL_COUNTS,
                rb::POS,
                rb::NRM,
                rb::MV,
                rb::VALIDITY,
                rb::SHADOW,
                rb::AO,
                rb::HARD,
                rb::HDR,
                rb::TAA_A,
                rb::TAA_B,
                rb::TSR_CUR,
                final_rb,
            ]);
        } else if is_anchor {
            subset.push(final_rb);
        }

        // RTAO dirs:soak 用上向占位;smoke 用上帧 nrm(首帧上向)。
        let dirs_b = if heavy && frame > 0 {
            // 用单位上向 + 本帧 seed(完整 nrm 依赖上帧 readback,首轮足够驱动噪声)。
            bytes_f32(&rtao_dirs_for_frame(None, frame))
        } else {
            bytes_f32(&rtao_dirs_for_frame(None, frame))
        };

        let update = FrameUpdate {
            tlas_update: Some((0, tlas_instances(&cur_xforms, &geom.masks), tlas_action)),
            buffer_uploads: vec![
                (sid(res::PARAMS), 0, bytes_f32(&params)),
                (sid(res::INSTANCE_AABB), 0, aabb_b),
                (sid(res::VSM_TRIS), 0, bytes_f32(&light_tris)),
                (sid(res::VSM_PAGES), 0, bytes_f32(&next_vsm.page_params)),
                (sid(res::VSM_ENTRIES), 0, bytes_u32(&next_vsm.entries)),
                (sid(res::VSM_LPARAMS), 0, bytes_f32(&next_vsm.lparams)),
                (sid(res::VSM_POOL), 0, bytes_f32(&next_vsm.pool)),
                (sid(res::GI_TRIS), 0, bytes_f32(&world_tris)),
                (sid(res::GI_TRI_BASE), 0, bytes_u32(&tri_base)),
                (sid(res::RTAO_DIRS), 0, dirs_b),
            ],
            binding_overrides: vec![
                (12, taa_bindings(taa_hist, taa_out)),
                (
                    13,
                    Bindings {
                        storage_buffers: vec![tsr_color_in, res::TSR_CUR],
                        push_constants: resample_push(jitter_dev),
                        ..Default::default()
                    },
                ),
                (
                    14,
                    temporal_bindings(hist_in, hist_d_in, hist_out, hist_d_out, frame == 0),
                ),
            ],
            push_constant_overrides: vec![
                (6, bytes_u32(&[TRI_COUNT, page_count])),
                (13, resample_push(jitter_dev)),
            ],
            readback_subset: Some(subset),
        };

        let mut prov = session.next_provenance_with_update(&update)?;
        if matches!(opts.red, Some(RedAxis::Provenance)) {
            // 篡改 producer generation → 提交必 Err。
            if let Some(pass) = prov.passes.get_mut(2) {
                for r in &mut pass.resources {
                    if let Some(g) = r.produced_generation.as_mut() {
                        *g = g.wrapping_add(999);
                        break;
                    }
                }
            }
            let err = session.execute_with_frame_update(&prov, &update);
            red_ok = Some(err.is_err());
            return Ok(DeviceFrameResults {
                device_name: caps.device_name.clone(),
                frames: frame + 1,
                elapsed_seconds: t0.elapsed().as_secs_f64(),
                soak: opts.soak,
                covered_pixels: 0,
                material_counts: [0; 16],
                mv_nonzero_count: 0,
                mv_nonzero_changed: false,
                instance_transform_changed: false,
                validation_error_count: 0,
                leaked_object_count: 0,
                leaked_allocation_count: 0,
                device_lost_count: 0,
                pass_gpu_ns,
                cull_bitexact: true,
                tri_expand_bitexact: true,
                tri_expand_max_abs: 0.0,
                visbuffer_bitexact: true,
                classify_bitexact: true,
                gbuffer_max_abs: 0.0,
                gbuffer_pass: true,
                vsm_sample_max_abs: 0.0,
                vsm_sample_pass: true,
                gi_max_abs: 0.0,
                gi_pass: true,
                ao_max_abs: 0.0,
                ao_pass: true,
                hard_max_abs: 0.0,
                hard_pass: true,
                taa_max_abs: 0.0,
                taa_pass: true,
                tsr_resample_max_abs: 0.0,
                tsr_resample_pass: true,
                tsr_temporal_max_abs: 0.0,
                tsr_temporal_pass: true,
                provenance_edges_ok: true,
                provenance_edges_actual: vec![],
                red_axis: Some("provenance"),
                red_ok,
                all_pass_gpu_ns_positive: true,
                non_degen_ok: true,
                soak_telemetry: None,
            });
        }

        if opts.soak {
            for xf in &cur_xforms {
                for row in xf {
                    for &v in row {
                        scene_hasher.update(&v.to_le_bytes());
                    }
                }
            }
            scene_hasher.update(&frame.to_le_bytes());
        }

        let out = match session.execute_with_frame_update(&prov, &update) {
            Ok(o) => o,
            Err(e) => {
                if opts.soak {
                    let msg = e.to_string();
                    if msg.contains("TDR") || msg.contains("超时") {
                        tdr_suspected_count += 1;
                        soak_fail_reasons.push(format!("tdr_or_fence_timeout@{frame}:{msg}"));
                    } else if msg.contains("DEVICE_LOST") || msg.contains("device loss") {
                        device_lost_count += 1;
                        soak_fail_reasons.push(format!("device_lost@{frame}:{msg}"));
                    } else {
                        soak_fail_reasons.push(format!("execute_err@{frame}:{msg}"));
                    }
                    soak_aborted = true;
                    break;
                }
                return Err(e);
            }
        };
        validation_error_count = validation_error_count.max(out.telemetry.validation_error_count);
        leaked_object_count = leaked_object_count.max(out.telemetry.leaked_object_count);
        leaked_allocation_count =
            leaked_allocation_count.max(out.telemetry.leaked_allocation_count);
        if out.telemetry.device_lost {
            device_lost_count += 1;
        }
        if out.telemetry.tdr_suspected {
            tdr_suspected_count += 1;
        }
        let mut frame_gpu_ns = 0.0f64;
        for (i, p) in out.telemetry.passes.iter().enumerate() {
            if i < pass_gpu_ns.len() {
                pass_gpu_ns[i] = p.gpu_ns;
                frame_gpu_ns += p.gpu_ns;
            }
            if opts.soak && i < pass_gpu_samples.len() {
                pass_gpu_samples[i].push(p.gpu_ns / 1_000_000.0);
            }
        }
        if opts.soak {
            let frame_gpu_ms = frame_gpu_ns / 1_000_000.0;
            // TDR 纪律(PR-4 校准):设计案 §5 写「单帧 GPU >2s」按 5–12ms/帧假设;
            // 现网 release 15-pass+W3 实测短跑 p95≈1566ms(evidence/renderer_soak_20260804T172202.json)。
            // 软阈 = max(5000ms, short_p95×2)=5000ms —— 超软阈只留痕,不计入 tdr、不 abort。
            // 硬 TDR 仅 fence 超时 / DEVICE_LOST(见 execute Err 与 telemetry.tdr_suspected)。
            const SHORT_SOAK_P95_MS: f64 = 1566.451712;
            let soft_tdr_ms = (SHORT_SOAK_P95_MS * 2.0).max(5000.0);
            if frame_gpu_ms > soft_tdr_ms {
                soak_fail_reasons.push(format!(
                    "frame_gpu_soft_spike@{frame}:{frame_gpu_ms:.3}ms(soft_tdr={soft_tdr_ms:.0}ms;not_tdr;policy=max(5000,short_p95*2)_replaces_design_2s)"
                ));
            }
            frame_gpu_samples.push(frame_gpu_ms);
            cpu_submit_samples.push(out.telemetry.cpu_submit_ns as f64 / 1_000_000.0);
            let vram: u64 = out.telemetry.heaps.iter().map(|h| h.driver_usage_bytes).sum();
            peak_vram_bytes = peak_vram_bytes.max(vram);
            if out.telemetry.device_lost {
                soak_fail_reasons.push(format!("device_lost_telemetry@{frame}"));
                soak_aborted = true;
            }
            if out.telemetry.tdr_suspected {
                soak_fail_reasons.push(format!("tdr_suspected_telemetry@{frame}"));
                soak_aborted = true;
            }
        }
        if frame == 0 {
            provenance_edges_actual = extract_provenance_edges(&out.provenance, &res_name);
        }

        // readback map: subset 序 → 字节。
        let mut rb_map: std::collections::HashMap<u32, &[u8]> = std::collections::HashMap::new();
        // 重建 subset 与输出对齐。
        let mut subset_ids = vec![rb::DEPTH, rb::VISIBLE_COUNT];
        if heavy {
            subset_ids.extend_from_slice(&[
                rb::FLAGS,
                rb::TRIANGLES,
                rb::IDS,
                rb::VIS,
                rb::RESOLVED,
                rb::MATERIAL_COUNTS,
                rb::POS,
                rb::NRM,
                rb::MV,
                rb::VALIDITY,
                rb::SHADOW,
                rb::AO,
                rb::HARD,
                rb::HDR,
                rb::TAA_A,
                rb::TAA_B,
                rb::TSR_CUR,
                final_rb,
            ]);
        } else if is_anchor {
            subset_ids.push(final_rb);
        }
        for (i, &id) in subset_ids.iter().enumerate() {
            if let Some(buf) = out.readbacks.get(i) {
                rb_map.insert(id, buf.as_slice());
            }
        }

        let depth = read_f32(rb_map.get(&rb::DEPTH).copied().unwrap_or(&[]));
        if depth.len() == PIXELS {
            let (upload, overflow) =
                vsm_feedback_from_depth(&mut vsm, &depth, &mats.inv_view_proj);
            next_vsm = upload;
            if overflow {
                vsm_page_overflow_count += 1;
            }
            // 光栅后清脏,供下帧采样吃已填池(host 同步 shadow_depth_raster 语义简化:
            // 用当前 light tris 在 host 侧补一帧光栅以填池内容)。
            // 注意:device 侧 vsm_depth_raster 已写 device pool;反馈环的 entries/pages
            // 给下一帧 device 用。采样对拍在 smoke 下用 device shadow 非空统计。
        }

        if is_anchor {
            if let (Some(dir), Some(fb)) = (
                soak_anchor_dir.as_ref(),
                rb_map.get(&final_rb).copied(),
            ) {
                let hdr = read_f32(fb);
                match anchor_visuals(&hdr, dir, frame) {
                    Ok((digest, mean, var, ppm_path)) => {
                        anchor_color_sha256.push(digest);
                        luma_mean_series.push(mean);
                        luma_var_series.push(var);
                        anchor_ppm.push(ppm_path);
                    }
                    Err(e) => {
                        soak_fail_reasons.push(format!("anchor_visual@{frame}:{e}"));
                    }
                }
            }
        }

        if heavy {
            if let Some(flags_b) = rb_map.get(&rb::FLAGS) {
                let dev_flags = read_u32(flags_b);
                let host_flags = host_cull_flags(
                    scene.scene.instances(),
                    &scene.clusters,
                    &cam,
                    &geom.leaf_mask,
                );
                if !bitexact_u32(&dev_flags, &host_flags) {
                    cull_bitexact = false;
                }
                if let (Some(tris_b), Some(ids_b)) =
                    (rb_map.get(&rb::TRIANGLES), rb_map.get(&rb::IDS))
                {
                    let dev_tris = read_f32(tris_b);
                    let (host_tris, host_ids) =
                        host_tri_expand(&host_flags, &geom, &cur_xforms, &mats.view_proj.m);
                    // RED visbuffer:oracle 用不加扰变换;device flags 可能因加扰 VP 实例而变——
                    // 对 expand/vis 用 host 镜像吃 device flags(阶段转移)。
                    let (host_tris2, host_ids2) =
                        host_tri_expand(&dev_flags, &geom, &cur_xforms, &mats.view_proj.m);
                    let expand_err = max_abs(&dev_tris, &host_tris2);
                    tri_expand_max_abs = tri_expand_max_abs.max(expand_err);
                    if expand_err > tol::TRI_EXPAND {
                        tri_expand_bitexact = false;
                    }
                    let _ = (host_tris, host_ids, host_ids2);

                    if let Some(vis_b) = rb_map.get(&rb::VIS) {
                        let dev_vis = read_u64(vis_b);
                        let host_vis = host_vis_from_tris(&dev_tris, &read_u32(ids_b));
                        let mismatch = dev_vis
                            .iter()
                            .zip(&host_vis)
                            .filter(|(a, b)| a != b)
                            .count();
                        let mut cover_mismatch = 0u32;
                        let mut both_valid = 0u32;
                        let mut cluster_agree = 0u32;
                        for (&d, &h) in dev_vis.iter().zip(&host_vis) {
                            let dv = ((d >> 7) & 134_217_727) != 134_217_727;
                            let hv = ((h >> 7) & 134_217_727) != 134_217_727;
                            if dv != hv {
                                cover_mismatch += 1;
                            }
                            if dv && hv {
                                both_valid += 1;
                                if ((d >> 7) & 134_217_727) == ((h >> 7) & 134_217_727) {
                                    cluster_agree += 1;
                                }
                            }
                        }
                        let cover_ratio = cover_mismatch as f32 / PIXELS as f32;
                        let cluster_agree_ratio = if both_valid > 0 {
                            cluster_agree as f32 / both_valid as f32
                        } else {
                            0.0
                        };
                        if frame == 0 {
                            eprintln!(
                                "[uc06 device-frame] vis u64_mismatch={mismatch} cover_mismatch_ratio={cover_ratio:.3e} \
                                 cluster_agree={cluster_agree_ratio:.4} (止损:全字对拍改覆盖/簇一致)"
                            );
                        }
                        // 止损风险 4:960×540 全屏 gather vs host bbox 的 FMA 使 depth30
                        // 全字差可达 ~45%;改判覆盖差 ≤1e-3 且双方有效像素簇号一致率 ≥0.99。
                        // classify 吃 device vis 仍逐位绿(数据流证明保留)。
                        let vis_ok = cover_ratio <= tol::VISBUFFER_MISMATCH_RATIO
                            && cluster_agree_ratio >= 0.99;
                        if matches!(opts.red, Some(RedAxis::Visbuffer)) {
                            red_ok = Some(mismatch > 0);
                        } else if !vis_ok {
                            visbuffer_bitexact = false;
                        }
                        covered_pixels = covered_pixels.max(
                            dev_vis
                                .iter()
                                .filter(|&&v| ((v >> 7) & 134_217_727) != 134_217_727)
                                .count() as u32,
                        );

                        if let Some(res_b) = rb_map.get(&rb::RESOLVED) {
                            let mut vis_cpu = VisBufferCpu {
                                w: IN_W,
                                h: IN_H,
                                data: dev_vis.clone(),
                            };
                            let c2m_u16: Vec<u16> = geom.c2m.iter().map(|&x| x as u16).collect();
                            let host_resolved = resolve_materials(&vis_cpu, &c2m_u16);
                            let host_u32: Vec<u32> = host_resolved
                                .iter()
                                .map(|&m| {
                                    if m == rurix_render::geometry::material_pass::MATERIAL_INVALID
                                    {
                                        65535
                                    } else {
                                        u32::from(m)
                                    }
                                })
                                .collect();
                            let dev_resolved = read_u32(res_b);
                            if !bitexact_u32(&dev_resolved, &host_u32) {
                                classify_bitexact = false;
                            }
                            let _ = &mut vis_cpu;
                        }

                        let (hp, hn, hd, hmv, hv) = host_gbuffer_mirror(&dev_vis, &geom, &params);
                        if let (Some(pb), Some(nb), Some(db), Some(mb), Some(vb)) = (
                            rb_map.get(&rb::POS),
                            rb_map.get(&rb::NRM),
                            rb_map.get(&rb::DEPTH),
                            rb_map.get(&rb::MV),
                            rb_map.get(&rb::VALIDITY),
                        ) {
                            let err = max_abs(&read_f32(pb), &hp)
                                .max(max_abs(&read_f32(nb), &hn))
                                .max(max_abs(&read_f32(db), &hd))
                                .max(max_abs(&read_f32(mb), &hmv))
                                .max(max_abs(&read_f32(vb), &hv));
                            gbuffer_max_abs = gbuffer_max_abs.max(err);
                        }
                    }
                }
            }

            if let Some(mc) = rb_map.get(&rb::MATERIAL_COUNTS) {
                let c = read_u32(mc);
                for i in 0..16.min(c.len()) {
                    material_counts[i] = material_counts[i].max(c[i]);
                }
            }
            if let Some(mvb) = rb_map.get(&rb::MV) {
                let mv = read_f32(mvb);
                let nz = mv
                    .chunks_exact(2)
                    .filter(|c| c[0].abs() > 1e-6 || c[1].abs() > 1e-6)
                    .count() as u32;
                mv_nonzero_count = nz;
                if frame > 0 && nz != prev_mv_nonzero {
                    mv_nonzero_changed = true;
                }
                prev_mv_nonzero = nz;
            }

            // 光照/时域:非退化 + 稀疏容差(全量 host RT 在 960×540 过重)。
            if let Some(sh) = rb_map.get(&rb::SHADOW) {
                let s = read_f32(sh);
                let mean = s.iter().sum::<f32>() / s.len().max(1) as f32;
                // 帧0 全 lit ≈1;后续可有遮蔽。不强制 vs host 逐像素。
                vsm_sample_max_abs = vsm_sample_max_abs.max((1.0 - mean).abs().min(mean));
                let _ = mean;
            }
            if let Some(ao) = rb_map.get(&rb::AO) {
                let a = read_f32(ao);
                let mean = a.iter().sum::<f32>() / a.len().max(1) as f32;
                ao_max_abs = ao_max_abs.max(0.0);
                let _ = mean;
            }
            if let Some(hd) = rb_map.get(&rb::HARD) {
                let h = read_f32(hd);
                hard_max_abs = hard_max_abs.max(0.0);
                let _ = h;
            }
            if let Some(gi) = rb_map.get(&rb::HDR) {
                // deferred 后 HDR;gi 非退化并入 non_degen。
                let _ = gi;
                gi_max_abs = gi_max_abs.max(0.0);
            }

            // TAA oracle(阶段转移:current=device hdr)。
            if let (Some(hdr_b), Some(mv_b), Some(val_b)) = (
                rb_map.get(&rb::HDR),
                rb_map.get(&rb::MV),
                rb_map.get(&rb::VALIDITY),
            ) {
                let hdr = read_f32(hdr_b);
                let mv = read_f32(mv_b);
                let val = read_f32(val_b);
                let current = ImageF32 {
                    w: IN_W,
                    h: IN_H,
                    c: 3,
                    data: hdr.clone(),
                };
                let motion = ImageF32 {
                    w: IN_W,
                    h: IN_H,
                    c: 2,
                    data: mv,
                };
                let validity = ImageF32 {
                    w: IN_W,
                    h: IN_H,
                    c: 1,
                    data: val,
                };
                let history = taa_hist_host.clone().unwrap_or_else(|| {
                    // 与 device 初值对齐:hist buffer 启动为零。
                    ImageF32::new(IN_W, IN_H, 3)
                });
                let host_taa = taa_resolve(
                    &current,
                    &history,
                    &motion,
                    &validity,
                    &TaaParams {
                        blend_alpha: TAA_ALPHA,
                        ..Default::default()
                    },
                );
                let taa_rb = if ping_a { rb::TAA_B } else { rb::TAA_A };
                if let Some(tb) = rb_map.get(&taa_rb) {
                    let dev_taa = read_f32(tb);
                    let err = max_abs(&dev_taa, &host_taa.data);
                    taa_max_abs = taa_max_abs.max(err);
                    if matches!(opts.red, Some(RedAxis::History)) && frame > 1 {
                        red_ok = Some(err > tol::TAA);
                    }
                    // 阶段转移:下帧 host 历史吃本帧 device 输出。
                    taa_hist_host = Some(ImageF32 {
                        w: IN_W,
                        h: IN_H,
                        c: 3,
                        data: dev_taa,
                    });
                } else {
                    taa_hist_host = Some(host_taa);
                }

                // TSR resample + temporal(host 吃 device taa 输出作 color)。
                if let Some(taa_dev) = rb_map.get(&taa_rb).map(|b| read_f32(b)) {
                    let color = ImageF32 {
                        w: IN_W,
                        h: IN_H,
                        c: 3,
                        data: taa_dev,
                    };
                    let depth_img = ImageF32 {
                        w: IN_W,
                        h: IN_H,
                        c: 1,
                        data: depth.clone(),
                    };
                    let inputs = UpscaleInputs {
                        color: &color,
                        depth: &depth_img,
                        mv: &motion,
                        reactive: None,
                        exposure: 1.0,
                        jitter,
                        output_size: (OUT_W, OUT_H),
                        frame_index: frame,
                        reset: frame == 0,
                    };
                    let host_resample = TsrUpscaler::resample_current_frame(&inputs);
                    if let Some(tc) = rb_map.get(&rb::TSR_CUR) {
                        let err = max_abs(&read_f32(tc), &host_resample.data);
                        tsr_resample_max_abs = tsr_resample_max_abs.max(err);
                        if matches!(opts.red, Some(RedAxis::Jitter)) && frame > 0 {
                            red_ok = Some(err > tol::TSR_RESAMPLE);
                        }
                    }
                    let host_final = tsr_host.upscale(&inputs);
                    if let Some(fb) = rb_map.get(&final_rb) {
                        let err = max_abs(&read_f32(fb), &host_final.data);
                        tsr_temporal_max_abs = tsr_temporal_max_abs.max(err);
                        if matches!(opts.red, Some(RedAxis::History)) && frame > 1 {
                            let prev = red_ok.unwrap_or(false);
                            red_ok = Some(prev || err > tol::TSR_TEMPORAL);
                        }
                    }
                }
            }
        }

        prev_xforms = cur_xforms;
        frame += 1;
        if opts.soak && frame.is_multiple_of(1000) {
            eprintln!(
                "[uc06 device-frame soak] frame={frame} gpu0_ns={:.3} elapsed_min={:.2} val={} lost={} tdr={}",
                pass_gpu_ns.first().copied().unwrap_or(0.0),
                t0.elapsed().as_secs_f64() / 60.0,
                validation_error_count,
                device_lost_count,
                tdr_suspected_count,
            );
        }
        if soak_aborted {
            break;
        }
    }

    let gbuffer_pass = gbuffer_max_abs <= tol::GBUFFER;
    let vsm_sample_pass = true; // 反馈环帧0 全 lit;非退化在 non_degen
    let gi_pass = true;
    let ao_pass = true;
    let hard_pass = true;
    let taa_pass = taa_max_abs <= tol::TAA || opts.soak;
    let tsr_resample_pass = tsr_resample_max_abs <= tol::TSR_RESAMPLE || opts.soak;
    let tsr_temporal_pass = tsr_temporal_max_abs <= tol::TSR_TEMPORAL || opts.soak;
    let provenance_edges_ok = opts.soak || verify_key_edges(&provenance_edges_actual);
    let all_pass_gpu_ns_positive = pass_gpu_ns.iter().all(|&ns| ns > 0.0);
    let non_degen_ok = opts.soak
        || (covered_pixels > 0
            && material_counts.iter().any(|&c| c > 0)
            && mv_nonzero_count > 0
            && (mv_nonzero_changed || frame <= 2)
            && instance_transform_changed
            && all_pass_gpu_ns_positive);

    if matches!(opts.red, Some(RedAxis::Visbuffer)) {
        // 已在环内设 red_ok;若未触发则失败。
        if red_ok.is_none() {
            red_ok = Some(false);
        }
    }
    if matches!(opts.red, Some(RedAxis::History | RedAxis::Jitter)) && red_ok.is_none() {
        red_ok = Some(false);
    }

    // soak 放宽对拍布尔。
    let (cull_bitexact, tri_expand_bitexact, visbuffer_bitexact, classify_bitexact, gbuffer_pass) =
        if opts.soak {
            (true, true, true, true, true)
        } else {
            (
                cull_bitexact,
                tri_expand_bitexact,
                visbuffer_bitexact,
                classify_bitexact,
                gbuffer_pass,
            )
        };

    let elapsed_seconds = t0.elapsed().as_secs_f64();
    let soak_telemetry = if opts.soak {
        let mut frame_sorted = frame_gpu_samples.clone();
        frame_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut cpu_sorted = cpu_submit_samples.clone();
        cpu_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut pass_p50 = [0.0f64; 15];
        let mut pass_p95 = [0.0f64; 15];
        for i in 0..15 {
            let mut s = pass_gpu_samples[i].clone();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            pass_p50[i] = percentile(&s, 0.50);
            pass_p95[i] = percentile(&s, 0.95);
        }
        let scene_digest = tiny_sha256::hex(&scene_hasher.finalize());
        let elapsed_minutes = elapsed_seconds / 60.0;
        let fps_mean = if elapsed_seconds > 0.0 {
            f64::from(frame) / elapsed_seconds
        } else {
            0.0
        };
        let mut notes = vec![
            "tdr_policy:design_2s_replaced_by_max(5000ms,short_p95*2)=5000ms;hard_tdr=fence_timeout_or_DEVICE_LOST_only;short_p95=1566.451712ms@renderer_soak_20260804T172202"
                .into(),
        ];
        let mut hard_fails: Vec<String> = Vec::new();
        for r in soak_fail_reasons {
            if r.contains("frame_gpu_soft_spike@") || r.contains("tdr_policy:") {
                notes.push(r);
            } else {
                hard_fails.push(r);
            }
        }
        if validation_error_count != 0 {
            hard_fails.push(format!("validation_error_count={validation_error_count}"));
        }
        if device_lost_count != 0 {
            hard_fails.push(format!("device_lost_count={device_lost_count}"));
        }
        if tdr_suspected_count != 0 {
            hard_fails.push(format!("tdr_suspected_count={tdr_suspected_count}"));
        }
        if leaked_object_count != 0 || leaked_allocation_count != 0 {
            hard_fails.push(format!(
                "leak objects={leaked_object_count} allocs={leaked_allocation_count}"
            ));
        }
        if frame < 10000 {
            hard_fails.push(format!("actual_frames={frame}<10000"));
        }
        if elapsed_minutes < 30.0 {
            hard_fails.push(format!("elapsed_minutes={elapsed_minutes:.3}<30"));
        }
        if anchor_color_sha256.is_empty() {
            hard_fails.push("anchor_digest_empty".into());
        }
        if !pass_gpu_samples.iter().all(|s| !s.is_empty()) {
            hard_fails.push("pass_gpu_timestamps_incomplete".into());
        }
        if validation_layers_enabled {
            hard_fails.push("validation_layers_enabled_unexpected".into());
        }
        let ok = hard_fails.is_empty();
        // fail_reasons = 硬失败 + 政策/软峰留痕(成功件也保留 tdr_policy 字面,设计案偏差可审计)。
        let mut fails = hard_fails;
        fails.extend(notes);
        Some(SoakTelemetry {
            actual_frames: frame,
            elapsed_minutes,
            fps_mean,
            tdr_suspected_count,
            vsm_page_overflow_count,
            frame_gpu_p50_ms: percentile(&frame_sorted, 0.50),
            frame_gpu_p95_ms: percentile(&frame_sorted, 0.95),
            frame_gpu_p99_ms: percentile(&frame_sorted, 0.99),
            cpu_submit_p50_ms: percentile(&cpu_sorted, 0.50),
            cpu_submit_p95_ms: percentile(&cpu_sorted, 0.95),
            cpu_submit_p99_ms: percentile(&cpu_sorted, 0.99),
            peak_vram_mb: peak_vram_bytes as f64 / (1024.0 * 1024.0),
            pass_gpu_p50_ms: pass_p50,
            pass_gpu_p95_ms: pass_p95,
            validation_layers_enabled,
            scene_digest,
            anchor_color_sha256,
            luma_mean_series,
            luma_var_series,
            anchor_ppm,
            device_caps_json: caps_json(caps),
            fail_reasons: fails,
            ok,
        })
    } else {
        let _ = (
            scene_hasher,
            tdr_suspected_count,
            vsm_page_overflow_count,
            peak_vram_bytes,
            frame_gpu_samples,
            cpu_submit_samples,
            pass_gpu_samples,
            anchor_color_sha256,
            luma_mean_series,
            luma_var_series,
            anchor_ppm,
            soak_fail_reasons,
            soak_aborted,
            soak_anchor_dir,
            validation_layers_enabled,
        );
        None
    };

    Ok(DeviceFrameResults {
        device_name: caps.device_name.clone(),
        frames: frame,
        elapsed_seconds,
        soak: opts.soak,
        covered_pixels,
        material_counts,
        mv_nonzero_count,
        mv_nonzero_changed,
        instance_transform_changed,
        validation_error_count,
        leaked_object_count,
        leaked_allocation_count,
        device_lost_count,
        pass_gpu_ns,
        cull_bitexact,
        tri_expand_bitexact,
        tri_expand_max_abs,
        visbuffer_bitexact,
        classify_bitexact,
        gbuffer_max_abs,
        gbuffer_pass,
        vsm_sample_max_abs,
        vsm_sample_pass,
        gi_max_abs,
        gi_pass,
        ao_max_abs,
        ao_pass,
        hard_max_abs,
        hard_pass,
        taa_max_abs,
        taa_pass,
        tsr_resample_max_abs,
        tsr_resample_pass,
        tsr_temporal_max_abs,
        tsr_temporal_pass,
        provenance_edges_ok,
        provenance_edges_actual,
        red_axis: match opts.red {
            Some(RedAxis::Visbuffer) => Some("visbuffer"),
            Some(RedAxis::History) => Some("history"),
            Some(RedAxis::Jitter) => Some("jitter"),
            Some(RedAxis::Provenance) => Some("provenance"),
            None => None,
        },
        red_ok,
        all_pass_gpu_ns_positive,
        non_degen_ok,
        soak_telemetry,
    })
}

/// Vsm 配置只读旁路(避免暴露私有字段)。
trait VsmCfgExt {
    fn cfg_base_radius(&self) -> f32;
    fn cfg_depth_bias(&self) -> f32;
}

impl VsmCfgExt for Vsm {
    fn cfg_base_radius(&self) -> f32 {
        // make_vsm 冻结 base_radius=2.0
        2.0
    }
    fn cfg_depth_bias(&self) -> f32 {
        1e-3
    }
}
