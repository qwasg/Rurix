//! Rurix 生产分发 fatbin 端到端单可执行产物(G1.5,契约 G-G1-5;Mini-RFC/MR-0005,RXS-0150/0151)。
//!
//! `kernels/saxpy.rx` 经 rurixc 全管线产 **PTX(fallback)+ 按架构预编 cubin(`ptxas -arch=sm_89`)**,
//! 由 `build.rs` 双变体嵌入本 EXE data 段(PTX `include_str!` + cubin `include_bytes!`,RXS-0150)。
//! main 走 [`Context::load_module_artifacts`] **fatbin 装载协商**(RXS-0151):查 device compute
//! capability → 命中按架构预编 cubin 即 `cuModuleLoadData`(首启免 JIT)、未命中 / cubin 拒绝降级
//! 保守 PTX fallback(既有 `cuModuleLoadDataEx` 版号梯子,RXS-0076/0077 语义 0-byte,降级而非
//! reject)→ launch → D2H → host 参考逐元素 f32 精确核对。
//!
//! 输出 `FATBIN_DIST: ok variant=<cubin|ptx> numeric=ok n=<N> sm=<sm>`(供 `ci/fatbin_dist_smoke.py`
//! device 段解析)。核对通过 exit 0;失配 exit 1。**降级 SKIP**(exit 0,真红绿在带 ptxas+GPU
//! runner):嵌入 PTX 为空(构建期无 clang/rurixc)/ 无 GPU。无 cubin(无 ptxas)→ 自动降级 PTX
//! fallback,行为等价 M8 PTX-only。

use core::ffi::c_void;
use std::process::ExitCode;

use rurix_rt::Context;
use rurix_rt::fatbin::{ArchKey, DeviceArtifactSet};

// build.rs 双变体产物:PTX fallback(include_str!)+ 按架构预编 cubin(include_bytes!,sm_89)。
const SAXPY_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/saxpy.ptx"));
const SAXPY_CUBIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/saxpy.sm_89.cubin"));
include!(concat!(env!("OUT_DIR"), "/saxpy_meta.rs"));

const CUBIN_ARCH: &str = "sm_89"; // build.rs CUBIN_ARCH(基线 sm_89,07 §7 / D-207)
const N: usize = 1 << 20;
const A: f32 = 2.5;
const BLOCK: u32 = 256;

fn gpu_available() -> bool {
    matches!(Context::device_count(), Ok(n) if n > 0)
}

fn main() -> ExitCode {
    if SAXPY_PTX.trim().is_empty() || SAXPY_KERNEL.is_empty() {
        eprintln!("[fatbin-saxpy] SKIP:构建期无 clang/rurixc 工具链,未嵌入 device PTX(降级 SKIP)");
        return ExitCode::SUCCESS;
    }
    if !gpu_available() {
        eprintln!("[fatbin-saxpy] SKIP:无可用 GPU/驱动(降级 SKIP)");
        return ExitCode::SUCCESS;
    }
    match run() {
        Ok(variant) => {
            println!("FATBIN_DIST: ok variant={variant} numeric=ok n={N} sm={CUBIN_ARCH}");
            eprintln!(
                "[fatbin-saxpy] PASS:fatbin 装载协商 + SAXPY 端到端真跑通过(variant={variant},{N} 元素 f32 精确相等)"
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[fatbin-saxpy] FAIL:{e}");
            ExitCode::FAILURE
        }
    }
}

/// 返回实际装载的变体名(`"cubin"` = 命中按架构预编 / `"ptx"` = 降级 fallback)。
fn run() -> Result<&'static str, String> {
    let x: Vec<f32> = (0..N).map(|i| (i as f32) * 0.5).collect();
    let y: Vec<f32> = (0..N).map(|i| (i as f32) * -1.25 + 3.0).collect();
    let expect: Vec<f32> = (0..N).map(|i| A * x[i] + y[i]).collect();

    let ctx = Context::new().map_err(|e| format!("创建 Context: {e:?}"))?;

    let mut dx = ctx
        .alloc::<f32>(N)
        .map_err(|e| format!("alloc dx: {e:?}"))?;
    let mut dy = ctx
        .alloc::<f32>(N)
        .map_err(|e| format!("alloc dy: {e:?}"))?;
    let d_out = ctx
        .alloc::<f32>(N)
        .map_err(|e| format!("alloc out: {e:?}"))?;
    dx.copy_from_host(&x).map_err(|e| format!("H2D x: {e:?}"))?;
    dy.copy_from_host(&y).map_err(|e| format!("H2D y: {e:?}"))?;

    // 分发产物变体集:PTX fallback 必存(保守兜底,RXS-0150)+ 按架构预编 cubin(非空时)。
    let mut set = DeviceArtifactSet::new(SAXPY_PTX);
    if !SAXPY_CUBIN.is_empty() {
        let sm = ArchKey::parse(CUBIN_ARCH).expect("CUBIN_ARCH 为合法 sm_ 架构键");
        set = set.with_cubin(sm, SAXPY_CUBIN.to_vec());
    }

    // fatbin 装载协商(RXS-0151):cubin 命中即用、未命中 / 拒绝降级保守 PTX fallback。
    let module = ctx
        .load_module_artifacts(&set)
        .map_err(|e| format!("fatbin 装载协商: {e:?}"))?;
    // negotiated_version() == "sm_xx" ⇒ 命中按架构预编 cubin;否则(PTX 版号)⇒ 降级 fallback。
    let variant = if module.negotiated_version().starts_with("sm_") {
        "cubin"
    } else {
        "ptx"
    };
    eprintln!(
        "[fatbin-saxpy] 装载协商:variant={variant},负载标识={},entry={}",
        module.negotiated_version(),
        SAXPY_KERNEL
    );

    let kernel = module
        .function(SAXPY_KERNEL)
        .map_err(|e| format!("cuModuleGetFunction {SAXPY_KERNEL}: {e:?}"))?;
    let stream = ctx
        .create_stream()
        .map_err(|e| format!("create_stream: {e:?}"))?;

    let mut p_out = d_out.device_ptr();
    let mut p_x = dx.device_ptr();
    let mut p_y = dy.device_ptr();
    let mut aa = A;
    let mut nn: u64 = N as u64;
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
    Ok(variant)
}
