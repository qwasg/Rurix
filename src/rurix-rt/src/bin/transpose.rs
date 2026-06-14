//! Rurix transpose 端到端真跑(M5.3,契约 D-M5-5)。
//!
//! `kernels/transpose.rx`(16x16 shared-tile 转置,2D ThreadCtx,atomics-free)经
//! rurixc 全管线产 PTX,build.rs 嵌入。main:H2D → 2D launch → D2H → 与 host 参考
//! (`dst[R*h+C]=src[C*w+R]`)逐元素精确核对(纯数据搬运,无浮点重排)。
//!
//! 降级 SKIP(exit 0):嵌入 PTX 空 / 无 GPU。

use core::ffi::c_void;
use std::process::ExitCode;

use rurix_rt::Context;

const TRANSPOSE_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/transpose.ptx"));
include!(concat!(env!("OUT_DIR"), "/transpose_meta.rs"));

// 输入 H 行 × W 列(非 16 整除,验证边界);输出 W 行 × H 列。
const W: usize = 200;
const H: usize = 150;
const TILE: u32 = 16;

fn gpu_available() -> bool {
    matches!(Context::device_count(), Ok(n) if n > 0)
}

fn main() -> ExitCode {
    if TRANSPOSE_PTX.trim().is_empty() || TRANSPOSE_KERNEL.is_empty() {
        eprintln!("[rurix-transpose] SKIP:未嵌入 device PTX(无 clang/CUDA 工具链)");
        return ExitCode::SUCCESS;
    }
    if !gpu_available() {
        eprintln!("[rurix-transpose] SKIP:无可用 GPU/驱动");
        return ExitCode::SUCCESS;
    }
    match run() {
        Ok(()) => {
            eprintln!("[rurix-transpose] PASS:transpose 真跑通过({H}x{W} → {W}x{H} 逐元素相等)");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[rurix-transpose] FAIL:{e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    // src: H 行 × W 列,行主序 src[row*W + col]
    let src: Vec<f32> = (0..H * W).map(|i| i as f32 * 0.5).collect();
    // 参考输出:dst (W 行 × H 列) dst[R*H + C] = src[C*W + R]
    let mut expect = vec![0f32; W * H];
    for r in 0..W {
        for c in 0..H {
            expect[r * H + c] = src[c * W + r];
        }
    }

    let ctx = Context::new().map_err(|e| format!("Context: {e:?}"))?;
    let mut dsrc = ctx
        .alloc::<f32>(H * W)
        .map_err(|e| format!("alloc src: {e:?}"))?;
    let ddst = ctx
        .alloc::<f32>(W * H)
        .map_err(|e| format!("alloc dst: {e:?}"))?;
    dsrc.copy_from_host(&src)
        .map_err(|e| format!("H2D src: {e:?}"))?;

    let module = ctx
        .load_module(TRANSPOSE_PTX)
        .map_err(|e| format!("load_module: {e:?}"))?;
    let kernel = module
        .function(TRANSPOSE_KERNEL)
        .map_err(|e| format!("function {TRANSPOSE_KERNEL}: {e:?}"))?;
    let stream = ctx.create_stream().map_err(|e| format!("stream: {e:?}"))?;

    let mut p_src = dsrc.device_ptr();
    let mut p_dst = ddst.device_ptr();
    let mut ww: u64 = W as u64;
    let mut hh: u64 = H as u64;
    let mut params: [*mut c_void; 4] = [
        (&raw mut p_src).cast::<c_void>(),
        (&raw mut p_dst).cast::<c_void>(),
        (&raw mut ww).cast::<c_void>(),
        (&raw mut hh).cast::<c_void>(),
    ];
    // grid.x 覆盖 W 列(xin<W),grid.y 覆盖 H 行(yin<H);block 16x16
    let gx = (W as u32).div_ceil(TILE);
    let gy = (H as u32).div_ceil(TILE);
    stream
        .launch(&kernel, [gx, gy, 1], [TILE, TILE, 1], &mut params)
        .map_err(|e| format!("launch: {e:?}"))?;
    stream.synchronize().map_err(|e| format!("sync: {e:?}"))?;

    let mut got = vec![0f32; W * H];
    ddst.copy_to_host(&mut got)
        .map_err(|e| format!("D2H dst: {e:?}"))?;

    for i in 0..W * H {
        if got[i] != expect[i] {
            return Err(format!("transpose @ {i}: got {} expect {}", got[i], expect[i]));
        }
    }
    Ok(())
}
