//! G2.4 UC-04 device 真跑支撑工具(非生产 codegen)。把 UC-04 deferred 着色器的 Rurix 源
//! 经图形=B DXIL 链 `rurixc::dxil_codegen::emit_dxil_b_container`(RXS-0171 body I/O 数据流
//! 降级 / RXS-0172 varying 用户语义名保名 / RXS-0173 fragment 输出 SV_Target# 忠实核对 /
//! 强制 `signature_gate`)落盘 DXIL 容器字节,供 `ci/dxil_uc04_device_smoke.py` 的 D3D12
//! graphics PSO 真机创建消费。G-G2-4 防降级硬门:device 消费的 DXIL 来自 Rurix 源经 rurixc
//! 图形=B 链,非手写 HLSL/DXIL。
//!
//! 用法 `emit_uc04_dxil <src.rx> <out.dxil>`(单文件单图形阶段;smoke 对 vs/fs 各调一次)。
//! 须 `dxil-backend` + `shader-stages` feature + pin B 工具(spirv-cross / dxc,经
//! `RURIX_SPIRV_CROSS` / `RURIX_DXC` 或 `RURIX_DXC_DIR`)。工具缺失 → 非零退出(device 真跑须 pin)。

use std::path::Path;
use std::process::ExitCode;

use rurixc::ast::ShaderStage;
use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: emit_uc04_dxil <src.rx> <out.dxil>");
        return ExitCode::FAILURE;
    }
    let src_path = Path::new(&args[1]);
    let out_path = Path::new(&args[2]);

    let src = match std::fs::read_to_string(src_path) {
        Ok(s) => s.replace("\r\n", "\n"),
        Err(e) => {
            eprintln!("emit_uc04_dxil: 读取源 {} 失败: {e}", src_path.display());
            return ExitCode::FAILURE;
        }
    };

    // 与 tests/dxil_golden.rs::graphics_stage_body 同源的前端编译(0 诊断 → device MIR)。
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    if diag.has_errors() {
        eprintln!(
            "emit_uc04_dxil: 源 {} 编译有诊断(图形语料应 0 诊断)",
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
            "emit_uc04_dxil: 源 {} 无 vertex/fragment 图形阶段根",
            src_path.display()
        );
        return ExitCode::FAILURE;
    };
    let stage = body.stage;

    match rurixc::dxil_codegen::emit_dxil_b_container(&body) {
        Ok(Some(dxil)) => {
            if let Some(parent) = out_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(out_path, &dxil) {
                eprintln!("emit_uc04_dxil: 写 DXIL {} 失败: {e}", out_path.display());
                return ExitCode::FAILURE;
            }
            println!(
                "emit_uc04_dxil: wrote {} bytes DXIL to {} (stage={stage:?})",
                dxil.len(),
                out_path.display()
            );
            ExitCode::SUCCESS
        }
        Ok(None) => {
            eprintln!(
                "emit_uc04_dxil: pin B 工具链(spirv-cross/dxc)不可用 → 无法产 DXIL 容器字节;\
                 device 真跑须 pin(set RURIX_DXC_DIR / RURIX_SPIRV_CROSS / RURIX_DXC)"
            );
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("emit_uc04_dxil: 图形=B 链 strict-only 失败(6xxx): {e:?}");
            ExitCode::FAILURE
        }
    }
}
