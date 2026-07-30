#![cfg(feature = "vulkan-backend")]

use std::path::{Path, PathBuf};
use std::process::Command;

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

const ATOMICS_U64: &str = "vk_atomics_w2_u64";

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

fn instructions(words: &[u32]) -> Vec<(u16, &[u32])> {
    let mut out = Vec::new();
    let mut at = 5usize;
    while at < words.len() {
        let first = words[at];
        let len = (first >> 16) as usize;
        assert!(len > 0 && at + len <= words.len(), "SPIR-V 指令长度非法");
        out.push(((first & 0xffff) as u16, &words[at + 1..at + len]));
        at += len;
    }
    out
}

fn capabilities(words: &[u32]) -> Vec<u32> {
    instructions(words)
        .into_iter()
        .filter(|(op, _)| *op == 17)
        .map(|(_, operands)| operands[0])
        .collect()
}

#[test]
fn atomics_w2_u64_emit_expected_types_capabilities_and_opcodes() {
    let words = emit(ATOMICS_U64);
    let insts = instructions(&words);
    assert!(
        insts
            .iter()
            .any(|(op, operands)| *op == 21 && operands.get(1) == Some(&64)),
        "产物缺 OpTypeInt 64"
    );
    for (opcode, name) in [
        (113, "OpUConvert"),
        (114, "OpSConvert"),
        (128, "OpIAdd"),
        (132, "OpIMul"),
        (177, "OpSLessThan"),
        (196, "OpShiftLeftLogical"),
        (197, "OpBitwiseOr"),
        (199, "OpBitwiseAnd"),
        (239, "OpAtomicUMax"),
    ] {
        assert!(
            insts.iter().any(|(op, _)| *op == opcode),
            "产物缺 {name}({opcode})"
        );
    }
    assert_eq!(
        capabilities(&words),
        vec![1, 11, 12],
        "W2 u64 原子应声明 Shader、Int64、Int64Atomics"
    );
    assert_eq!(words[1], 0x0001_0000, "compute 须保持 SPIR-V 1.0");
}

#[test]
fn int64_capabilities_are_demand_driven() {
    let src = "kernel fn int64_scalar(x: u64) {\n    let _y = x + 1u64;\n}\nfn main() {}\n";
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(!diag.has_errors(), "前端应 0 诊断:{:?}", diag.emitted());
    let words = rurixc::vulkan_codegen::build_and_emit_vulkan(&cx, "int64_scalar")
        .unwrap_or_else(|| panic!("u64 标量 Vulkan codegen 未产出:{:?}", diag.emitted()));
    assert_eq!(
        capabilities(&words),
        vec![1, 11],
        "纯 u64 标量只应追加 Int64，不应声明 Int64Atomics"
    );
}

#[test]
fn f64_compute_remains_rx6026() {
    let src = "kernel fn f64_rejected(x: f64) {\n    let _y = x + 1.0f64;\n}\nfn main() {}\n";
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(
        !diag.has_errors(),
        "f64 负例前端应通过，由 Vulkan codegen 拒绝:{:?}",
        diag.emitted()
    );
    assert!(
        rurixc::vulkan_codegen::build_and_emit_vulkan(&cx, "f64_rejected").is_none(),
        "f64 compute 不应产出 SPIR-V"
    );
    let codes: Vec<u16> = diag
        .emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect();
    assert_eq!(codes, vec![6026], "f64 compute 应精确报 RX6026");
}

#[test]
fn compute_w2_u64_passes_spirv_val() {
    let Some(tool) = rurixc::toolchain::locate_spirv_val() else {
        eprintln!("[SKIP] spirv-val 定位失败(dev-env degrade,非 fake pass)");
        return;
    };
    let words = emit(ATOMICS_U64);
    let bytes = rurixc::vulkan_codegen::words_to_bytes(&words);
    let path = std::env::temp_dir().join(format!(
        "rurix_compute_w2_{}_{}.spv",
        std::process::id(),
        ATOMICS_U64
    ));
    std::fs::write(&path, bytes).expect("写临时 SPIR-V");
    let output = Command::new(&tool)
        .arg("--target-env")
        .arg("vulkan1.0")
        .arg(&path)
        .output();
    let _ = std::fs::remove_file(&path);
    match output {
        Err(_) => eprintln!("[SKIP] spirv-val 不可执行(dev-env degrade)"),
        Ok(out) if out.status.success() => {
            eprintln!(
                "[OK] spirv-val --target-env vulkan1.0 accept:{}",
                ATOMICS_U64
            );
        }
        Ok(out) => panic!(
            "spirv-val 拒绝 {}:stdout={} stderr={}",
            ATOMICS_U64,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    }
}
