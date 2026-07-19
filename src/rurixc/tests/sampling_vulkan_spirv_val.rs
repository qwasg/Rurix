//! G3.3 PR-S3(RFC-0013 §4.B / 验收门 G-G3-3):采样超集 device 模式着色器 `.rx` → Vulkan
//! 原生 SPIR-V(`emit_spirv_body_vulkan`,provenance=false,Vk-native set-per-class 绑定装饰)
//! → `spirv-val --target-env vulkan1.0` accept。库级全链兑现(图形 emit 无 CLI `--emit`
//! 通道,镜像 dxil_corpus::sample_superset_source_reaches_new_opcodes;device 数值判据归
//! bin/sampling_modes + 步骤 63 owner 本机)。
//!
//! spirv-val 三态:工具在位 → 每模式 .spv accept(严格 Vulkan1.0);缺工具 / 不可执行 →
//! SKIP(dev-env degrade,非 fake pass;退出码判定非 grep)。**这是 harness device 真跑前
//! 唯一的 SPIR-V 合法性机验闸门**——绑定装饰经 Vk-native set-per-class 与
//! `run_graphics_offscreen_v2` 的 `plan_descriptor_sets` 分配律对齐(SRV→set1/UAV→set2/
//! Sampler→set3),两 crate 分立镜像同一律。

#![cfg(feature = "shader-stages")]

use std::path::{Path, PathBuf};
use std::process::Command;

use rurixc::ast::ShaderStage;
use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

/// conformance/dxil/graphics/accept(CARGO_MANIFEST_DIR = src/rurixc → repo root)。
fn accept_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/dxil/graphics/accept")
}

/// PR-S3 device 模式着色器集(vertex ×2 + fragment ×5;每 .rx 单阶段根)。fragment 复用:
/// sampling_sample_lod_fs 服务模式 ①〔mip 选层〕/ ⑥〔wrap-vs-clamp〕/ ⑦〔多分量〕。
const MODE_SHADERS: &[&str] = &[
    "sampling_fullscreen_vs",
    "sampling_fetch_vs",
    "sampling_sample_lod_fs",
    "sampling_load_fs",
    "sampling_gather_fs",
    "sampling_cmp_fs",
    "sampling_storage_fs",
];

/// `.rx` → Vulkan 原生 SPIR-V 字节(0 诊断门 + emit_spirv_body_vulkan)。
fn emit_vulkan_spv(stem: &str) -> Vec<u8> {
    let path = accept_dir().join(format!("{stem}.rx"));
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {stem}.rx 失败: {e}"))
        .replace("\r\n", "\n");
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(
        !diag.has_errors(),
        "{stem} 应 0 诊断: {:?}",
        diag.emitted()
            .iter()
            .filter_map(|d| d.code)
            .collect::<Vec<_>>()
    );
    let bodies = cx.device_mir_crate();
    let body = bodies
        .iter()
        .find(|b| matches!(b.stage, Some(ShaderStage::Vertex | ShaderStage::Fragment)))
        .unwrap_or_else(|| panic!("{stem} 无 vertex/fragment 图形阶段根"));
    let words = rurixc::dxil_spirv::emit_spirv_body_vulkan(body.stage.unwrap(), body)
        .unwrap_or_else(|e| panic!("{stem} emit_spirv_body_vulkan 失败: {e:?}"));
    let mut bytes = Vec::with_capacity(words.len() * 4);
    for w in &words {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    bytes
}

/// 每模式 .rx 均产非空 Vulkan SPIR-V(magic 0x07230203)——工具无关,恒跑。
#[test]
fn sampling_modes_emit_vulkan_spirv() {
    for stem in MODE_SHADERS {
        let bytes = emit_vulkan_spv(stem);
        assert!(
            bytes.len() >= 20,
            "{stem} Vulkan SPIR-V 过短({} 字节)",
            bytes.len()
        );
        assert_eq!(
            &bytes[0..4],
            &0x0723_0203u32.to_le_bytes(),
            "{stem} Vulkan SPIR-V magic 不符"
        );
    }
}

/// 每模式 Vulkan SPIR-V 过 `spirv-val --target-env vulkan1.0`(工具在位 accept / 缺工具
/// SKIP 三态;退出码判定)。device 真跑前唯一的 SPIR-V 合法性机验闸门。
#[test]
fn sampling_modes_pass_spirv_val() {
    let Some(tool) = rurixc::toolchain::locate_spirv_val() else {
        eprintln!("[SKIP] spirv-val 定位失败(dev-env degrade,非 fake pass)");
        return;
    };
    let mut validated = 0usize;
    for stem in MODE_SHADERS {
        let bytes = emit_vulkan_spv(stem);
        let spv = std::env::temp_dir().join(format!("rurix_s3_{}_{stem}.spv", std::process::id()));
        if std::fs::write(&spv, &bytes).is_err() {
            eprintln!("[SKIP] 写临时 .spv 失败(dev-env degrade)");
            return;
        }
        let out = Command::new(&tool)
            .arg("--target-env")
            .arg("vulkan1.0")
            .arg(&spv)
            .output();
        let _ = std::fs::remove_file(&spv);
        match out {
            // spawn 失败(工具不在 PATH)→ SKIP(PATH-defer 兜底,非 fake)。
            Err(_) => {
                eprintln!("[SKIP] spirv-val 不可执行(dev-env degrade)");
                return;
            }
            Ok(o) if o.status.success() => {
                validated += 1;
                eprintln!("[OK] spirv-val --target-env vulkan1.0 accept: {stem}");
            }
            Ok(o) => panic!(
                "spirv-val 拒绝 {stem}: stdout={} stderr={}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            ),
        }
    }
    assert_eq!(
        validated,
        MODE_SHADERS.len(),
        "spirv-val 应对全部 {} 模式 accept",
        MODE_SHADERS.len()
    );
}
