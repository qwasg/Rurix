//! Rurix SAXPY 端到端单可执行产物(M4.4,契约 D-M4-5 / G-M4-1 真跑通道;06 §5.2)。
//!
//! `kernels/saxpy.rx` 经 rurixc 全管线产 PTX,由 `build.rs` 嵌入本 EXE data 段
//! (`include_str!`)。main 走运行时 `rurix-rt` 经典内存路径:装载协商 → alloc →
//! H2D → `cuLaunchKernel` → D2H → host 参考逐元素 f32 精确核对(SAXPY 无重排,
//! mul.rn+add.rn 与两步舍入逐位一致)。核对通过 exit 0;失配 exit 1(对齐 CI 步骤
//! 20 真跑铁律)。
//!
//! **降级 SKIP**(均 exit 0,真红绿在带 clang+GPU 的 self-hosted runner):
//! - 嵌入 PTX 为空(构建期无 clang/rurixc,`build.rs` 写空哨兵);
//! - 无 GPU/驱动(`Context::device_count()==0`)。

use core::ffi::c_void;
use std::process::ExitCode;

use rurix_rt::Context;

// build.rs 产物:嵌入的 PTX(clang NVPTX 后端)+ ptx_kernel 入口符号名。
const SAXPY_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/saxpy.ptx"));
include!(concat!(env!("OUT_DIR"), "/saxpy_meta.rs"));

const N: usize = 1 << 20; // 1,048,576 元素(端到端正确性档;measured 基准见 bench/)
const A: f32 = 2.5;
const BLOCK: u32 = 256;

fn gpu_available() -> bool {
    matches!(Context::device_count(), Ok(n) if n > 0)
}

fn main() -> ExitCode {
    if SAXPY_PTX.trim().is_empty() || SAXPY_KERNEL.is_empty() {
        eprintln!(
            "[rurix-saxpy] SKIP:构建期无 clang/rurixc 工具链,未嵌入 device PTX(降级 SKIP;真红绿在带 clang+GPU runner)"
        );
        return ExitCode::SUCCESS;
    }
    if !gpu_available() {
        eprintln!("[rurix-saxpy] SKIP:无可用 GPU/驱动(降级 SKIP)");
        return ExitCode::SUCCESS;
    }
    match run() {
        Ok(()) => {
            eprintln!("[rurix-saxpy] PASS:Rurix SAXPY 端到端真跑通过({N} 元素 f32 精确相等)");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[rurix-saxpy] FAIL:{e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let x: Vec<f32> = (0..N).map(|i| (i as f32) * 0.5).collect();
    let y: Vec<f32> = (0..N).map(|i| (i as f32) * -1.25 + 3.0).collect();
    // host 参考:两步舍入(mul 后 add),与 device mul.rn+add.rn 逐位一致
    let expect: Vec<f32> = (0..N).map(|i| A * x[i] + y[i]).collect();

    let ctx = Context::new().map_err(|e| format!("创建 Context: {e:?}"))?;

    let mut dx = ctx.alloc::<f32>(N).map_err(|e| format!("alloc dx: {e:?}"))?;
    let mut dy = ctx.alloc::<f32>(N).map_err(|e| format!("alloc dy: {e:?}"))?;
    let d_out = ctx.alloc::<f32>(N).map_err(|e| format!("alloc out: {e:?}"))?;
    dx.copy_from_host(&x).map_err(|e| format!("H2D x: {e:?}"))?;
    dy.copy_from_host(&y).map_err(|e| format!("H2D y: {e:?}"))?;

    let module = ctx
        .load_module(SAXPY_PTX)
        .map_err(|e| format!("装载协商 + cuModuleLoadDataEx: {e:?}"))?;
    eprintln!(
        "[rurix-saxpy] 装载协商通过,.version = {},entry = {}",
        module.negotiated_version(),
        SAXPY_KERNEL
    );
    let kernel = module
        .function(SAXPY_KERNEL)
        .map_err(|e| format!("cuModuleGetFunction {SAXPY_KERNEL}: {e:?}"))?;
    let stream = ctx.create_stream().map_err(|e| format!("create_stream: {e:?}"))?;

    // launch 实参(kernel 形参顺序:out:ptr, x:ptr, y:ptr, a:f32, n:usize/i64)
    let mut p_out = d_out.device_ptr();
    let mut p_x = dx.device_ptr();
    let mut p_y = dy.device_ptr();
    let mut aa = A;
    let mut nn: u64 = N as u64; // usize → i64(8 字节)
    let mut params: [*mut c_void; 5] = [
        (&raw mut p_out).cast::<c_void>(),
        (&raw mut p_x).cast::<c_void>(),
        (&raw mut p_y).cast::<c_void>(),
        (&raw mut aa).cast::<c_void>(),
        (&raw mut nn).cast::<c_void>(),
    ];
    let grid = (N as u32).div_ceil(BLOCK);
    stream
        .launch(&kernel, [grid, 1, 1], [BLOCK, 1, 1], &mut params)
        .map_err(|e| format!("cuLaunchKernel: {e:?}"))?;
    stream
        .synchronize()
        .map_err(|e| format!("cuStreamSynchronize: {e:?}"))?;

    let mut got = vec![0f32; N];
    d_out
        .copy_to_host(&mut got)
        .map_err(|e| format!("D2H out: {e:?}"))?;

    for i in 0..N {
        if got[i] != expect[i] {
            return Err(format!(
                "SAXPY 逐元素核对失败 @ {i}: got {} expect {}",
                got[i], expect[i]
            ));
        }
    }
    Ok(())
}
