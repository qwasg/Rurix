//! G3.4 PR-K1(RFC-0013 §4.C / 验收门 G-G3-4):bindless 无界表动态索引着色器 `.rx` →
//! Vulkan 原生 SPIR-V(`emit_spirv_body_vulkan`,provenance=false)→ `spirv-val
//! --target-env vulkan1.2` accept。descriptor indexing(`RuntimeDescriptorArray` /
//! `ShaderNonUniform` / `SPV_EXT_descriptor_indexing`)= Vulkan 1.2 core,故校验环境
//! vulkan1.2(承 RXS-0212)。device 数值判据(四象限动态索引 == 四色 / 篡改注册序换位
//! RED)归 bin/bindless_modes + 步骤 64 owner 本机(判据阈值 TODO)。
//!
//! spirv-val 三态:工具在位 → .spv accept(严格 Vulkan1.2);缺工具 / 不可执行 → SKIP
//! (dev-env degrade,非 fake pass;退出码判定非 grep)。**harness device 真跑前唯一的
//! bindless SPIR-V 合法性机验闸门**。

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

/// G3.4 bindless device 模式着色器(四象限 vertex + 无界表动态非均匀索引采样 fragment,
/// = `bin/bindless_modes` harness 消费的完整着色器对)。
const BINDLESS_SHADERS: &[&str] = &["bindless_quadrant_vs", "bindless_sample_fs"];

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

/// bindless `.rx` 产非空 Vulkan SPIR-V(magic 0x07230203)——工具无关,恒跑。
//@ spec: RXS-0234
#[test]
fn bindless_emits_vulkan_spirv() {
    for stem in BINDLESS_SHADERS {
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

/// bindless Vulkan SPIR-V 过 `spirv-val --target-env vulkan1.2`(工具在位 accept / 缺工具
/// SKIP 三态;退出码判定)。harness device 真跑前唯一的 bindless SPIR-V 合法性机验闸门。
//@ spec: RXS-0232, RXS-0234
#[test]
fn bindless_passes_spirv_val() {
    let Some(tool) = rurixc::toolchain::locate_spirv_val() else {
        eprintln!("[SKIP] spirv-val 定位失败(dev-env degrade,非 fake pass)");
        return;
    };
    let mut validated = 0usize;
    for stem in BINDLESS_SHADERS {
        let bytes = emit_vulkan_spv(stem);
        let spv = std::env::temp_dir().join(format!("rurix_k1_{}_{stem}.spv", std::process::id()));
        if std::fs::write(&spv, &bytes).is_err() {
            eprintln!("[SKIP] 写临时 .spv 失败(dev-env degrade)");
            return;
        }
        let out = Command::new(&tool)
            .arg("--target-env")
            .arg("vulkan1.2")
            .arg(&spv)
            .output();
        let _ = std::fs::remove_file(&spv);
        match out {
            Err(_) => {
                eprintln!("[SKIP] spirv-val 不可执行(dev-env degrade)");
                return;
            }
            Ok(o) if o.status.success() => {
                validated += 1;
                eprintln!("[OK] spirv-val --target-env vulkan1.2 accept: {stem}");
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
        BINDLESS_SHADERS.len(),
        "spirv-val 应对全部 {} bindless 模式 accept",
        BINDLESS_SHADERS.len()
    );
}
