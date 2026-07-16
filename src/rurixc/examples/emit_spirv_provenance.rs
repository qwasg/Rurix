//! mb1 Phase 3 red_self_test 支撑工具(非生产 codegen;RXS-0210)。把图形着色阶段的 Rurix
//! 源经 `dxil_spirv::emit_spirv_body`(**provenance=true**,DXIL 路)落盘 SPIR-V 字节——即
//! **带** `UserSemantic` + `OpExtension SPV_GOOGLE_hlsl_functionality1` 的变体,供
//! `ci/vulkan_graphics_smoke.py` 的 red_self_test 反证:同一管线喂此变体 → `vkCreateShaderModule`
//! 按 VUID-VkShaderModuleCreateInfo-pCode-08742 拒(证方案 B 前坑真实);而 `spirv-val` 仍
//! **接受**此变体(证修复是「去装饰」非「产非法 SPIR-V」——validation-vs-runtime 诚实性)。
//!
//! 生产 `--target vulkan` 走 `emit_spirv_body_vulkan`(provenance=false,去装饰),非本工具。
//! 用法 `emit_spirv_provenance <src.rx> <out.spv>`(单文件单图形阶段)。

use std::path::Path;
use std::process::ExitCode;

use rurixc::ast::ShaderStage;
use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: emit_spirv_provenance <src.rx> <out.spv>");
        return ExitCode::FAILURE;
    }
    let src_path = Path::new(&args[1]);
    let out_path = Path::new(&args[2]);

    let src = match std::fs::read_to_string(src_path) {
        Ok(s) => s.replace("\r\n", "\n"),
        Err(e) => {
            eprintln!(
                "emit_spirv_provenance: 读取源 {} 失败: {e}",
                src_path.display()
            );
            return ExitCode::FAILURE;
        }
    };

    // 与 tests/dxil_golden.rs / emit_uc04_dxil 同源的前端编译(0 诊断 → device MIR)。
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    if diag.has_errors() {
        eprintln!(
            "emit_spirv_provenance: 源 {} 编译有诊断(图形语料应 0 诊断)",
            src_path.display()
        );
        return ExitCode::FAILURE;
    }
    let bodies = cx.device_mir_crate();
    let Some(body) = bodies
        .into_iter()
        .find(|b| matches!(b.stage, Some(ShaderStage::Vertex | ShaderStage::Fragment)))
    else {
        eprintln!(
            "emit_spirv_provenance: 源 {} 无 vertex/fragment 图形阶段根",
            src_path.display()
        );
        return ExitCode::FAILURE;
    };
    let stage = body.stage;

    // provenance=true(DXIL 路,带 UserSemantic + SPV_GOOGLE);与 emit_spirv_body_vulkan 对照。
    let words = match rurixc::dxil_spirv::emit_spirv_body(stage.unwrap(), &body) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("emit_spirv_provenance: SPIR-V 降级失败: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut bytes = Vec::with_capacity(words.len() * 4);
    for w in &words {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(out_path, &bytes) {
        eprintln!("emit_spirv_provenance: 写 {} 失败: {e}", out_path.display());
        return ExitCode::FAILURE;
    }
    println!(
        "emit_spirv_provenance: wrote {} bytes SPIR-V (provenance=true) to {} (stage={stage:?})",
        bytes.len(),
        out_path.display()
    );
    ExitCode::SUCCESS
}
