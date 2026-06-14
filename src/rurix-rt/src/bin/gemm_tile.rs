//! Rurix tiled GEMM 端到端真跑(M5.3,契约 D-M5-5 / G-M5-1 通道)。
//!
//! `kernels/gemm_tile.rx`(经典 16x16 shared-memory tiling,2D ThreadCtx,**不触
//! Tensor Core**,SG-002 维持 not_triggered)经 rurixc 全管线产 PTX,build.rs 嵌入。
//! main:H2D A/B → 2D launch → D2H C → 与 host 参考(三重循环 f64 累加)相对容差核对。
//!
//! 降级 SKIP(exit 0):嵌入 PTX 空 / 无 GPU。

use core::ffi::c_void;
use std::process::ExitCode;

use rurix_rt::Context;

const GEMM_TILE_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/gemm_tile.ptx"));
include!(concat!(env!("OUT_DIR"), "/gemm_tile_meta.rs"));

// C[M,N] = A[M,K] * B[K,N](非 16 整除,验证边界)。
const M: usize = 100;
const N: usize = 80;
const K: usize = 70;
const TILE: u32 = 16;
const REL_TOL: f64 = 1e-3;

fn gpu_available() -> bool {
    matches!(Context::device_count(), Ok(n) if n > 0)
}

fn main() -> ExitCode {
    if GEMM_TILE_PTX.trim().is_empty() || GEMM_TILE_KERNEL.is_empty() {
        eprintln!("[rurix-gemm] SKIP:未嵌入 device PTX(无 clang/CUDA 工具链)");
        return ExitCode::SUCCESS;
    }
    if !gpu_available() {
        eprintln!("[rurix-gemm] SKIP:无可用 GPU/驱动");
        return ExitCode::SUCCESS;
    }
    match run() {
        Ok(maxerr) => {
            eprintln!(
                "[rurix-gemm] PASS:tiled GEMM 真跑通过({M}x{K} * {K}x{N},max rel err={maxerr:.2e})"
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[rurix-gemm] FAIL:{e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<f64, String> {
    let a: Vec<f32> = (0..M * K).map(|i| ((i % 7) as f32) * 0.1 + 0.05).collect();
    let b: Vec<f32> = (0..K * N).map(|i| ((i % 5) as f32) * 0.2 + 0.1).collect();
    // host 参考:C[m,n] = sum_k A[m,k]*B[k,n](f64 累加)
    let mut expect = vec![0f32; M * N];
    for row in 0..M {
        for col in 0..N {
            let mut acc = 0f64;
            for kk in 0..K {
                acc += a[row * K + kk] as f64 * b[kk * N + col] as f64;
            }
            expect[row * N + col] = acc as f32;
        }
    }

    let ctx = Context::new().map_err(|e| format!("Context: {e:?}"))?;
    let mut da = ctx
        .alloc::<f32>(M * K)
        .map_err(|e| format!("alloc a: {e:?}"))?;
    let mut db = ctx
        .alloc::<f32>(K * N)
        .map_err(|e| format!("alloc b: {e:?}"))?;
    let dc = ctx
        .alloc::<f32>(M * N)
        .map_err(|e| format!("alloc c: {e:?}"))?;
    da.copy_from_host(&a).map_err(|e| format!("H2D a: {e:?}"))?;
    db.copy_from_host(&b).map_err(|e| format!("H2D b: {e:?}"))?;

    let module = ctx
        .load_module(GEMM_TILE_PTX)
        .map_err(|e| format!("load_module: {e:?}"))?;
    let kernel = module
        .function(GEMM_TILE_KERNEL)
        .map_err(|e| format!("function {GEMM_TILE_KERNEL}: {e:?}"))?;
    let stream = ctx.create_stream().map_err(|e| format!("stream: {e:?}"))?;

    let mut p_a = da.device_ptr();
    let mut p_b = db.device_ptr();
    let mut p_c = dc.device_ptr();
    let mut mm: u64 = M as u64;
    let mut nn: u64 = N as u64;
    let mut kk: u64 = K as u64;
    let mut params: [*mut c_void; 6] = [
        (&raw mut p_a).cast::<c_void>(),
        (&raw mut p_b).cast::<c_void>(),
        (&raw mut p_c).cast::<c_void>(),
        (&raw mut mm).cast::<c_void>(),
        (&raw mut nn).cast::<c_void>(),
        (&raw mut kk).cast::<c_void>(),
    ];
    // grid.x 覆盖 N 列(col<N),grid.y 覆盖 M 行(row<M);block 16x16
    let gx = (N as u32).div_ceil(TILE);
    let gy = (M as u32).div_ceil(TILE);
    stream
        .launch(&kernel, [gx, gy, 1], [TILE, TILE, 1], &mut params)
        .map_err(|e| format!("launch: {e:?}"))?;
    stream.synchronize().map_err(|e| format!("sync: {e:?}"))?;

    let mut got = vec![0f32; M * N];
    dc.copy_to_host(&mut got)
        .map_err(|e| format!("D2H c: {e:?}"))?;

    let mut maxerr = 0f64;
    for i in 0..M * N {
        let denom = (expect[i] as f64).abs().max(1.0);
        let err = (got[i] as f64 - expect[i] as f64).abs() / denom;
        maxerr = maxerr.max(err);
        if err > REL_TOL {
            return Err(format!(
                "gemm @ {i}: got {} expect {} (rel err {err:.2e})",
                got[i], expect[i]
            ));
        }
    }
    Ok(maxerr)
}
