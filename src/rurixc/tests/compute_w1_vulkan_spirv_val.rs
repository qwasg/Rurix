#![cfg(feature = "vulkan-backend")]

use std::path::{Path, PathBuf};
use std::process::Command;

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

const ATOMICS: &str = "vk_atomics_w1";
const STORAGE_IMAGE: &str = "vk_storage_image_w1";

fn accept_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/vulkan/accept")
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

#[test]
fn storage_image_w1_is_format_qualified_image_write() {
    let words = emit(STORAGE_IMAGE);
    let ops = opcodes(&words);
    assert!(ops.contains(&25), "产物缺 OpTypeImage");
    assert!(ops.contains(&99), "产物缺 OpImageWrite");
    assert_eq!(words[1], 0x0001_0000, "compute 须保持 SPIR-V 1.0");

    let mut capabilities = Vec::new();
    let mut at = 5usize;
    while at < words.len() {
        let first = words[at];
        let len = (first >> 16) as usize;
        if (first & 0xffff) as u16 == 17 {
            capabilities.push(words[at + 1]);
        }
        at += len;
    }
    assert_eq!(
        capabilities,
        vec![1],
        "compute W1 应仅声明 Capability Shader"
    );
}

#[test]
fn compute_w1_passes_spirv_val() {
    let Some(tool) = rurixc::toolchain::locate_spirv_val() else {
        eprintln!("[SKIP] spirv-val 定位失败(dev-env degrade,非 fake pass)");
        return;
    };
    for stem in [ATOMICS, STORAGE_IMAGE] {
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
