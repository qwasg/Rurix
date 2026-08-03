#![cfg(feature = "vulkan-backend")]

use std::path::{Path, PathBuf};
use std::process::Command;

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

const ATOMICS: &str = "vk_atomics_w1";
/// G7.5.0:compute storage image 语料自 `accept/` 归位 `reject/` —— RXS-0223 §4.0-2
/// 把 `TextureRw2D<F>` 阶段面逐字冻结为 **fragment + raygen**,compute 不在其中,
/// 故 `RX3013` 是**符合冻结 spec 的正确拒绝**(见下 `storage_image_in_compute_is_rejected`)。
const STORAGE_IMAGE: &str = "vk_storage_image_w1";

fn accept_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/vulkan/accept")
}

fn reject_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/vulkan/reject")
}

fn emit(stem: &str) -> Vec<u32> {
    let path = accept_dir().join(format!("{stem}.rx"));
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {} 失败:{e}", path.display()))
        .replace("\r\n", "\n");
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(
        !diag.has_errors(),
        "{stem} 前端应 0 诊断:{:?}",
        diag.emitted()
    );
    let words = rurixc::vulkan_codegen::build_and_emit_vulkan(&cx, stem)
        .unwrap_or_else(|| panic!("{stem} Vulkan codegen 未产出:{:?}", diag.emitted()));
    assert!(
        !diag.has_errors(),
        "{stem} Vulkan codegen 应 0 诊断:{:?}",
        diag.emitted()
    );
    words
}

fn opcodes(words: &[u32]) -> Vec<u16> {
    let mut out = Vec::new();
    let mut at = 5usize;
    while at < words.len() {
        let first = words[at];
        let len = (first >> 16) as usize;
        assert!(len > 0 && at + len <= words.len(), "SPIR-V 指令长度非法");
        out.push((first & 0xffff) as u16);
        at += len;
    }
    out
}

#[test]
fn atomics_w1_emit_expected_opcodes() {
    let words = emit(ATOMICS);
    let ops = opcodes(&words);
    for (opcode, name) in [
        (229, "OpAtomicExchange"),
        (230, "OpAtomicCompareExchange"),
        (234, "OpAtomicIAdd"),
        (235, "OpAtomicISub"),
        (236, "OpAtomicSMin"),
        (237, "OpAtomicUMin"),
        (238, "OpAtomicSMax"),
        (239, "OpAtomicUMax"),
        (240, "OpAtomicAnd"),
        (241, "OpAtomicOr"),
        (242, "OpAtomicXor"),
    ] {
        assert!(ops.contains(&opcode), "产物缺 {name}({opcode})");
    }
}

/// G7.5.0(冻结 spec 面的**正确拒绝**机器断言):`TextureRw2D<F>` 在 compute
/// (`kernel fn`)签名中 → `RX3013`,且**不产** SPIR-V(strict-only,不静默降级)。
///
/// 依据 = spec/shader_stages.md RXS-0223 §4.0-2「`TextureRw2D<F>` 阶段面 …… 首期
/// 阶段列 = **fragment + raygen**」。本测试取代原
/// `storage_image_w1_is_format_qualified_image_write`(该测试绕过 driver 的
/// `shader_stages` 关卡直取 codegen,把一条 **spec 面非法** 的形态断言成 accept,
/// 与恒跑步 `ci/vulkan_codegen_smoke.py` 的 accept 段互相矛盾)。
#[test]
fn storage_image_in_compute_is_rejected() {
    let path = reject_dir().join(format!("{STORAGE_IMAGE}.rx"));
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {} 失败:{e}", path.display()))
        .replace("\r\n", "\n");
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    // 阶段面关卡在 AST 层(RXS-0153~0156,cargo feature `shader-stages`),与
    // driver 同序:`check_shader_stages` 先于 codegen(driver.rs 阶段化中止口径)。
    cx.check_shader_stages();
    cx.check_crate();
    assert!(
        diag.has_errors(),
        "compute 签名 TextureRw2D 应被 RXS-0223 阶段面拒(RX3013),实测 0 诊断"
    );
    assert!(
        diag.emitted()
            .iter()
            .any(|d| d.code == Some(rurixc::diag::ErrorCode(3013))),
        "应发 RX3013(资源句柄位置违例),实测:{:?}",
        diag.emitted()
    );
    assert!(
        rurixc::vulkan_codegen::build_and_emit_vulkan(&cx, STORAGE_IMAGE).is_none(),
        "前端已拒的语料不得产出 SPIR-V(strict-only)"
    );
}

#[test]
fn compute_w1_passes_spirv_val() {
    let Some(tool) = rurixc::toolchain::locate_spirv_val() else {
        eprintln!("[SKIP] spirv-val 定位失败(dev-env degrade,非 fake pass)");
        return;
    };
    for stem in [ATOMICS] {
        let words = emit(stem);
        let bytes = rurixc::vulkan_codegen::words_to_bytes(&words);
        let path = std::env::temp_dir().join(format!(
            "rurix_compute_w1_{}_{stem}.spv",
            std::process::id()
        ));
        std::fs::write(&path, bytes).expect("写临时 SPIR-V");
        let output = Command::new(&tool)
            .arg("--target-env")
            .arg("vulkan1.0")
            .arg(&path)
            .output();
        let _ = std::fs::remove_file(&path);
        match output {
            Err(_) => {
                eprintln!("[SKIP] spirv-val 不可执行(dev-env degrade)");
                return;
            }
            Ok(out) if out.status.success() => {
                eprintln!("[OK] spirv-val --target-env vulkan1.0 accept:{stem}");
            }
            Ok(out) => panic!(
                "spirv-val 拒绝 {stem}:stdout={} stderr={}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        }
    }
}
