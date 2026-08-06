use std::path::{Path, PathBuf};

use rurixc::ast::ShaderStage;
use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

const KERNELS: &[&str] = &[
    "cull",
    "visbuffer_sw_u64",
    "classify_resolve",
    "vsm_page_mark",
    "taa",
    // G7.4 W3c 三效果核(RD-038「屏幕探针 GI」/「RTAO 硬阴影」;共用同一真实 TLAS)。
    "gi_probe",
    "rtao",
    "hard_shadow",
    // G7.5 RD-038 余项(「VSM 深度」页内光栅 + 采样;「TAA-TSR」的 TSR 腿)。
    "vsm_depth_raster",
    "vsm_depth_raster_mv",
    "vsm_sample",
    "tsr_resample",
    // G7.6 PR-1:TSR 时域臂孤立腿(闪烁 EMA / 重投影 / YCoCg AABB / 混合 / 五件套)。
    "tsr_temporal",
    // G8.5b M24:TSR 生产契约主核 + retired ring 维护核。
    "tsr_contract",
    "tsr_retire",
    // G7.6 PR-2:帧链 glue(设计 §1.2;既有 9 kernel 字节不动)。
    "frame_clear",
    "cull_frame",
    "tri_expand",
    "gbuffer_resolve",
    "deferred_shade",
    // G8.4 M37:驻留页 FNV-1a(u32 word) digest。
    "stream_consume_digest",
    // G8.5b M25:CAS/EASU 空间超分(副 UpscaleBackend device 腿)。
    "cas_upscale",
];

/// G7.5b HW 光栅图形着色对(RXS-0301~0303;门 G-G7-7):源 = conformance accept
/// 语料**同源文本**(设计 §2.2「语料即 uc06 实装内核」),经 `emit_spirv_body_vulkan`
/// 两遍编译新路(FS 走第二遍扩展白名单,VS 恒走第一遍零漂移路径)产 `.spv`。
/// `(源语料 stem, 产物 stem, 阶段)`;产物路径对齐 KERNELS 的 OUT_DIR 平铺惯例。
const GRAPHICS_SHADERS: &[(&str, &str, ShaderStage)] = &[
    (
        "vk_hw_raster_visbuffer_vs",
        "visbuffer_hw_vs",
        ShaderStage::Vertex,
    ),
    (
        "vk_hw_raster_visbuffer_fs",
        "visbuffer_hw_fs",
        ShaderStage::Fragment,
    ),
];

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("out dir"));
    for stem in KERNELS {
        let source = manifest.join("kernels").join(format!("{stem}.rx"));
        println!("cargo:rerun-if-changed={}", source.display());
        let target = out.join(format!("{stem}.spv"));
        if std::env::var_os("CARGO_FEATURE_VULKAN").is_some() {
            let bytes = compile(&source, stem);
            std::fs::write(&target, bytes)
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        } else {
            std::fs::write(&target, [])
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        }
    }
    let accept_dir = manifest.join("../../conformance/vulkan/accept");
    for (src_stem, out_stem, stage) in GRAPHICS_SHADERS {
        let source = accept_dir.join(format!("{src_stem}.rx"));
        println!("cargo:rerun-if-changed={}", source.display());
        let target = out.join(format!("{out_stem}.spv"));
        if std::env::var_os("CARGO_FEATURE_VULKAN").is_some() {
            let bytes = compile_graphics(&source, src_stem, *stage);
            std::fs::write(&target, bytes)
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        } else {
            std::fs::write(&target, [])
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        }
    }
}

fn compile(source: &Path, stem: &str) -> Vec<u8> {
    let src = std::fs::read_to_string(source)
        .unwrap_or_else(|e| panic!("read {}: {e}", source.display()));
    assert!(
        !src.contains('\r'),
        "{} must use LF line endings",
        source.display()
    );
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_launch();
    cx.check_crate_patterns();
    cx.check_views();
    cx.check_shared_barrier();
    cx.check_consteval();
    assert!(
        !diag.has_errors(),
        "{} frontend diagnostics: {:?}",
        source.display(),
        diag.emitted()
    );
    let words = rurixc::vulkan_codegen::build_and_emit_vulkan(&cx, stem)
        .unwrap_or_else(|| panic!("{} Vulkan codegen: {:?}", source.display(), diag.emitted()));
    assert!(
        !diag.has_errors(),
        "{} Vulkan diagnostics: {:?}",
        source.display(),
        diag.emitted()
    );
    rurixc::vulkan_codegen::words_to_bytes(&words)
}

/// 图形阶段 `.rx` → Vulkan 原生 SPIR-V(镜像 `tests/hw_raster_vulkan_spirv_val.rs`
/// 的阶段化前端序:`check_shader_stages` 先于 typeck〔RXS-0153~0156〕;发射走
/// `dxil_spirv::emit_spirv_body_vulkan` 两遍编译,provenance=false)。
fn compile_graphics(source: &Path, stem: &str, stage: ShaderStage) -> Vec<u8> {
    let src = std::fs::read_to_string(source)
        .unwrap_or_else(|e| panic!("read {}: {e}", source.display()))
        .replace("\r\n", "\n");
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    cx.check_shader_stages();
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(
        !diag.has_errors(),
        "{} frontend diagnostics: {:?}",
        source.display(),
        diag.emitted()
    );
    let bodies = cx.device_mir_crate();
    let res = cx.resolutions();
    let body = bodies
        .iter()
        .find(|b| b.stage == Some(stage))
        .unwrap_or_else(|| panic!("{stem} 无 {stage:?} 图形阶段根"));
    let words = rurixc::dxil_spirv::emit_spirv_body_vulkan(stage, body, &res)
        .unwrap_or_else(|e| panic!("{stem} emit_spirv_body_vulkan: {e:?}"));
    rurixc::vulkan_codegen::words_to_bytes(&words)
}
