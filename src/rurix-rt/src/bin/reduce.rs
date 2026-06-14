//! Rurix reduce 端到端真跑(M5.3,契约 D-M5-5 / G-M5-1 通道)。
//!
//! `kernels/reduce.rx`(block 级 shared 树形归约,atomics-free)经 rurixc 全管线
//! (含 libdevice 链接关卡)产 PTX,build.rs 嵌入。main:H2D → launch(每 block 产
//! 一 partial)→ D2H partials → host 合并求和 → 与 host 参考(f64 累加)相对容差
//! 核对(浮点重排,BENCH_PROTOCOL 容差口径)。
//!
//! 降级 SKIP(exit 0):嵌入 PTX 空(无 clang/CUDA)/ 无 GPU。

use core::ffi::c_void;
use std::process::ExitCode;

use rurix_rt::Context;

const REDUCE_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/reduce.ptx"));
include!(concat!(env!("OUT_DIR"), "/reduce_meta.rs"));

const N: usize = 1 << 20;
const BLOCK: u32 = 256;
const REL_TOL: f64 = 1e-5;

fn gpu_available() -> bool {
    matches!(Context::device_count(), Ok(n) if n > 0)
}

fn main() -> ExitCode {
    if REDUCE_PTX.trim().is_empty() || REDUCE_KERNEL.is_empty() {
        eprintln!("[rurix-reduce] SKIP:未嵌入 device PTX(无 clang/CUDA 工具链)");
        return ExitCode::SUCCESS;
    }
    if !gpu_available() {
        eprintln!("[rurix-reduce] SKIP:无可用 GPU/驱动");
        return ExitCode::SUCCESS;
    }
    match run() {
        Ok((got, expect)) => {
            eprintln!(
                "[rurix-reduce] PASS:reduce 真跑通过({N} 元素,sum={got} 参考={expect})"
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[rurix-reduce] FAIL:{e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(f64, f64), String> {
    let src: Vec<f32> = (0..N).map(|i| ((i % 13) as f32) * 0.25).collect();
    let expect: f64 = src.iter().map(|&v| v as f64).sum();

    let grid = (N as u32).div_ceil(BLOCK);
    let nblocks = grid as usize;

    let ctx = Context::new().map_err(|e| format!("Context: {e:?}"))?;
    let mut dsrc = ctx.alloc::<f32>(N).map_err(|e| format!("alloc src: {e:?}"))?;
    let dpart = ctx
        .alloc::<f32>(nblocks)
        .map_err(|e| format!("alloc partials: {e:?}"))?;
    dsrc.copy_from_host(&src)
        .map_err(|e| format!("H2D src: {e:?}"))?;

    let module = ctx
        .load_module(REDUCE_PTX)
        .map_err(|e| format!("load_module: {e:?}"))?;
    let kernel = module
        .function(REDUCE_KERNEL)
        .map_err(|e| format!("function {REDUCE_KERNEL}: {e:?}"))?;
    let stream = ctx.create_stream().map_err(|e| format!("stream: {e:?}"))?;

    let mut p_src = dsrc.device_ptr();
    let mut p_part = dpart.device_ptr();
    let mut nn: u64 = N as u64;
    let mut params: [*mut c_void; 3] = [
        (&raw mut p_src).cast::<c_void>(),
        (&raw mut p_part).cast::<c_void>(),
        (&raw mut nn).cast::<c_void>(),
    ];
    stream
        .launch(&kernel, [grid, 1, 1], [BLOCK, 1, 1], &mut params)
        .map_err(|e| format!("launch: {e:?}"))?;
    stream.synchronize().map_err(|e| format!("sync: {e:?}"))?;

    let mut partials = vec![0f32; nblocks];
    dpart
        .copy_to_host(&mut partials)
        .map_err(|e| format!("D2H partials: {e:?}"))?;
    let got: f64 = partials.iter().map(|&v| v as f64).sum();

    let denom = expect.abs().max(1.0);
    if (got - expect).abs() / denom > REL_TOL {
        return Err(format!("reduce 求和偏差超容差:got {got} expect {expect}"));
    }
    Ok((got, expect))
}
