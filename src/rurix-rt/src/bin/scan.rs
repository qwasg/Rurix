//! Rurix scan 端到端真跑(M5.3,契约 D-M5-5)。
//!
//! `kernels/scan.rx`(block 级 Hillis-Steele inclusive 前缀和,shared+barrier,
//! atomics-free)经 rurixc 全管线产 PTX,build.rs 嵌入。main:H2D → launch → D2H →
//! 与 host 参考(逐 block inclusive scan)相对容差核对。
//!
//! 降级 SKIP(exit 0):嵌入 PTX 空 / 无 GPU。

use core::ffi::c_void;
use std::process::ExitCode;

use rurix_rt::Context;

const SCAN_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/scan.ptx"));
include!(concat!(env!("OUT_DIR"), "/scan_meta.rs"));

const N: usize = 1 << 20;
const BLOCK: usize = 256;
const REL_TOL: f64 = 1e-5;

fn gpu_available() -> bool {
    matches!(Context::device_count(), Ok(n) if n > 0)
}

fn main() -> ExitCode {
    if SCAN_PTX.trim().is_empty() || SCAN_KERNEL.is_empty() {
        eprintln!("[rurix-scan] SKIP:未嵌入 device PTX(无 clang/CUDA 工具链)");
        return ExitCode::SUCCESS;
    }
    if !gpu_available() {
        eprintln!("[rurix-scan] SKIP:无可用 GPU/驱动");
        return ExitCode::SUCCESS;
    }
    match run() {
        Ok(maxerr) => {
            eprintln!("[rurix-scan] PASS:scan 真跑通过({N} 元素,max rel err={maxerr:.2e})");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[rurix-scan] FAIL:{e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<f64, String> {
    let src: Vec<f32> = (0..N).map(|i| ((i % 11) as f32) * 0.5 + 0.25).collect();
    // host 参考:逐 block(BLOCK)inclusive scan(与 kernel 同口径)
    let mut expect = vec![0f32; N];
    for base in (0..N).step_by(BLOCK) {
        let end = (base + BLOCK).min(N);
        let mut acc = 0f64;
        for i in base..end {
            acc += src[i] as f64;
            expect[i] = acc as f32;
        }
    }

    let ctx = Context::new().map_err(|e| format!("Context: {e:?}"))?;
    let mut dsrc = ctx.alloc::<f32>(N).map_err(|e| format!("alloc src: {e:?}"))?;
    let ddst = ctx.alloc::<f32>(N).map_err(|e| format!("alloc dst: {e:?}"))?;
    dsrc.copy_from_host(&src)
        .map_err(|e| format!("H2D src: {e:?}"))?;

    let module = ctx
        .load_module(SCAN_PTX)
        .map_err(|e| format!("load_module: {e:?}"))?;
    let kernel = module
        .function(SCAN_KERNEL)
        .map_err(|e| format!("function {SCAN_KERNEL}: {e:?}"))?;
    let stream = ctx.create_stream().map_err(|e| format!("stream: {e:?}"))?;

    let mut p_src = dsrc.device_ptr();
    let mut p_dst = ddst.device_ptr();
    let mut nn: u64 = N as u64;
    let mut params: [*mut c_void; 3] = [
        (&raw mut p_src).cast::<c_void>(),
        (&raw mut p_dst).cast::<c_void>(),
        (&raw mut nn).cast::<c_void>(),
    ];
    let grid = (N as u32).div_ceil(BLOCK as u32);
    stream
        .launch(&kernel, [grid, 1, 1], [BLOCK as u32, 1, 1], &mut params)
        .map_err(|e| format!("launch: {e:?}"))?;
    stream.synchronize().map_err(|e| format!("sync: {e:?}"))?;

    let mut got = vec![0f32; N];
    ddst.copy_to_host(&mut got)
        .map_err(|e| format!("D2H dst: {e:?}"))?;

    let mut maxerr = 0f64;
    for i in 0..N {
        let denom = (expect[i] as f64).abs().max(1.0);
        let err = (got[i] as f64 - expect[i] as f64).abs() / denom;
        maxerr = maxerr.max(err);
        if err > REL_TOL {
            return Err(format!(
                "scan @ {i}: got {} expect {} (rel err {err:.2e})",
                got[i], expect[i]
            ));
        }
    }
    Ok(maxerr)
}
