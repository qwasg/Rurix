//! Rurix 互操作 FFI 边界(M8.1,D-M8-1;spec/interop.md RXS-0122 ~ RXS-0125)。
//!
//! UC-01 PyTorch 瓶颈算子替换的实现层:复用 M5 自研 kernel(SAXPY / Reduction /
//! GEMM,经 build.rs 全管线产 PTX 嵌入),经 `__cuda_array_interface__` v3 / DLPack
//! 双协议从 PyTorch CUDA 张量取得**设备指针**(由上层 nanobind 绑定零拷贝抽取),
//! 在与 PyTorch **共享的 device primary context**([`rurix_rt::Context::from_primary`])
//! 内借用其设备内存([`rurix_rt::Context::from_device_ptr`],Drop 不释放)直接 launch
//! ——无主机往返、无设备内存重分配(零拷贝接入)。
//!
//! **分层**(M8_CONTRACT §5 / RXS-0125):
//! - 本 crate safe API([`saxpy`]/[`reduce`]/[`gemm`])对上**全 safe**(无 `unsafe`
//!   出现在签名);unsafe 仅在借用外部设备指针处(每块 `// SAFETY:`,注册见
//!   `unsafe-audit/rurix-interop.md`)。
//! - [`ffi`] 模块为 C ABI 导出层(`extern "C"`,nanobind 经 scikit-build-core 链接,
//!   产 PYD),返回 [`i32`] 错误码(0 = 成功;互操作诊断段位 RX7013~RX7015)。
//!
//! 新段位错误码(07 §5 段位语义,7xxx 链接/工具链续接,只追加、含义冻结):
//! - [`RX_INTEROP_UNSUPPORTED_PROTOCOL`](7013):协议不支持(对象未暴露 CAI v3 /
//!   DLPack,由 nanobind 绑定层抽取设备指针失败时上抛)。
//! - [`RX_INTEROP_INVALID_DEVICE_PTR`](7014):设备指针非法(空指针 / 非设备地址)。
//! - [`RX_INTEROP_SHAPE_MISMATCH`](7015):形状不匹配(维度为 0 / 算子维度不相容)。

pub mod ffi;

use rurix_rt::Context;

mod kernels {
    //! build.rs 产物:复用 M5 自研 kernel 的嵌入 PTX(clang NVPTX 后端)+ 入口符号名。
    pub const SAXPY_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/saxpy.ptx"));
    pub const REDUCE_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/reduce.ptx"));
    pub const GEMM_TILE_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/gemm_tile.ptx"));
    include!(concat!(env!("OUT_DIR"), "/saxpy_meta.rs"));
    include!(concat!(env!("OUT_DIR"), "/reduce_meta.rs"));
    include!(concat!(env!("OUT_DIR"), "/gemm_tile_meta.rs"));
}

/// 成功(C ABI 返回码,07 §5)。
pub const RX_OK: i32 = 0;
/// RX7013:互操作协议不支持(对象未暴露 `__cuda_array_interface__` v3 / DLPack)。
pub const RX_INTEROP_UNSUPPORTED_PROTOCOL: i32 = 7013;
/// RX7014:设备指针非法(空指针 / 非设备地址)。
pub const RX_INTEROP_INVALID_DEVICE_PTR: i32 = 7014;
/// RX7015:形状不匹配(维度为 0 / 算子维度不相容)。
pub const RX_INTEROP_SHAPE_MISMATCH: i32 = 7015;
/// 运行时/驱动失败(非互操作诊断段位:PTX 装载 / launch / 无 GPU / 无嵌入 PTX)。
pub const RX_INTEROP_RUNTIME: i32 = -1;

const BLOCK: u32 = 256;
const TILE: u32 = 16;

/// 校验设备指针非空(RXS-0123 / RXS-0124:零拷贝设备指针消费的合法性前置)。
/// 任一为 0 → [`RX_INTEROP_INVALID_DEVICE_PTR`]。纯 CPU 校验,先于任何 GPU 调用。
fn validate_ptrs(ptrs: &[u64]) -> Result<(), i32> {
    if ptrs.contains(&0) {
        return Err(RX_INTEROP_INVALID_DEVICE_PTR);
    }
    Ok(())
}

/// 校验维度全为正(RXS-0123:shape 合法性)。任一为 0 → [`RX_INTEROP_SHAPE_MISMATCH`]。
fn validate_dims(dims: &[usize]) -> Result<(), i32> {
    if dims.contains(&0) {
        return Err(RX_INTEROP_SHAPE_MISMATCH);
    }
    Ok(())
}

/// SAXPY 算子替换:`out[i] = a * x[i] + y[i]`(零拷贝接入 PyTorch,复用 M5 saxpy
/// kernel;RXS-0123 / RXS-0124)。`out`/`x`/`y` 为 PyTorch CUDA 张量设备指针
/// (`n` 个 `f32`,同一 primary context);返回 0 = 成功 / RX7014 / RX7015 / 运行时失败。
pub fn saxpy(out: u64, x: u64, y: u64, a: f32, n: usize) -> i32 {
    if let Err(code) = validate_ptrs(&[out, x, y]) {
        return code;
    }
    if let Err(code) = validate_dims(&[n]) {
        return code;
    }
    match run_saxpy(out, x, y, a, n) {
        Ok(()) => RX_OK,
        Err(_) => RX_INTEROP_RUNTIME,
    }
}

/// Reduction 算子替换:`out[0] = Σ x[i]`(零拷贝接入 PyTorch,复用 M5 reduce
/// block 级 shared 树形归约 kernel;RXS-0123 / RXS-0124)。`x` 为输入设备指针
/// (`n` 个 `f32`),`out` 为 1 元素标量设备指针(求和结果)。
pub fn reduce(out: u64, x: u64, n: usize) -> i32 {
    if let Err(code) = validate_ptrs(&[out, x]) {
        return code;
    }
    if let Err(code) = validate_dims(&[n]) {
        return code;
    }
    match run_reduce(out, x, n) {
        Ok(()) => RX_OK,
        Err(_) => RX_INTEROP_RUNTIME,
    }
}

/// GEMM 算子替换:`C[M,N] = A[M,K] · B[K,N]`(行主序,零拷贝接入 PyTorch,复用 M5
/// gemm_tile 16x16 shared tiling kernel,**不触 Tensor Core** SG-002;RXS-0123 /
/// RXS-0124)。`a`/`b`/`c` 为 PyTorch CUDA 张量设备指针(行主序 `f32`)。
pub fn gemm(c: u64, a: u64, b: u64, m: usize, n: usize, k: usize) -> i32 {
    if let Err(code) = validate_ptrs(&[c, a, b]) {
        return code;
    }
    if let Err(code) = validate_dims(&[m, n, k]) {
        return code;
    }
    match run_gemm(c, a, b, m, n, k) {
        Ok(()) => RX_OK,
        Err(_) => RX_INTEROP_RUNTIME,
    }
}

fn run_saxpy(out: u64, x: u64, y: u64, a: f32, n: usize) -> Result<(), String> {
    if kernels::SAXPY_PTX.trim().is_empty() || kernels::SAXPY_KERNEL.is_empty() {
        return Err("no embedded saxpy PTX".to_owned());
    }
    let ctx = Context::from_primary(0).map_err(|e| format!("from_primary: {e:?}"))?;
    // SAFETY: out/x/y 为 PyTorch CUDA 张量设备指针(经 CAI v3 / DLPack 零拷贝抽取),
    // 同一 device primary context 内有效、可读写、容纳 n 个 f32;借用期内 PyTorch
    // 持有不释放(借用缓冲 Drop 不 free,所有权留外部 deleter,RXS-0124)。
    let d_out = unsafe { ctx.from_device_ptr::<f32>(out, n) };
    // SAFETY: 同上(x 输入张量设备指针,n 个 f32)。
    let d_x = unsafe { ctx.from_device_ptr::<f32>(x, n) };
    // SAFETY: 同上(y 输入张量设备指针,n 个 f32)。
    let d_y = unsafe { ctx.from_device_ptr::<f32>(y, n) };
    let module = ctx
        .load_module(kernels::SAXPY_PTX)
        .map_err(|e| format!("load_module: {e:?}"))?;
    let kernel = module
        .function(kernels::SAXPY_KERNEL)
        .map_err(|e| format!("function: {e:?}"))?;
    let stream = ctx.create_stream().map_err(|e| format!("stream: {e:?}"))?;

    let mut p_out = d_out.device_ptr();
    let mut p_x = d_x.device_ptr();
    let mut p_y = d_y.device_ptr();
    let mut aa = a;
    let mut nn: u64 = n as u64;
    let mut params: [*mut core::ffi::c_void; 5] = [
        (&raw mut p_out).cast(),
        (&raw mut p_x).cast(),
        (&raw mut p_y).cast(),
        (&raw mut aa).cast(),
        (&raw mut nn).cast(),
    ];
    let grid = (n as u32).div_ceil(BLOCK);
    stream
        .launch(&kernel, [grid, 1, 1], [BLOCK, 1, 1], &mut params)
        .map_err(|e| format!("launch: {e:?}"))?;
    stream.synchronize().map_err(|e| format!("sync: {e:?}"))
}

fn run_reduce(out: u64, x: u64, n: usize) -> Result<(), String> {
    if kernels::REDUCE_PTX.trim().is_empty() || kernels::REDUCE_KERNEL.is_empty() {
        return Err("no embedded reduce PTX".to_owned());
    }
    let ctx = Context::from_primary(0).map_err(|e| format!("from_primary: {e:?}"))?;
    let grid = (n as u32).div_ceil(BLOCK);
    let nblocks = grid as usize;
    // SAFETY: x 为 PyTorch CUDA 张量输入设备指针(n 个 f32),同一 primary context
    // 内有效、借用期内不释放(RXS-0124)。
    let d_x = unsafe { ctx.from_device_ptr::<f32>(x, n) };
    // 归约 partials 为本层拥有的临时设备缓冲(owned,Drop 释放;非借用)。
    let dpart = ctx
        .alloc::<f32>(nblocks)
        .map_err(|e| format!("alloc partials: {e:?}"))?;
    let module = ctx
        .load_module(kernels::REDUCE_PTX)
        .map_err(|e| format!("load_module: {e:?}"))?;
    let kernel = module
        .function(kernels::REDUCE_KERNEL)
        .map_err(|e| format!("function: {e:?}"))?;
    let stream = ctx.create_stream().map_err(|e| format!("stream: {e:?}"))?;

    let mut p_x = d_x.device_ptr();
    let mut p_part = dpart.device_ptr();
    let mut nn: u64 = n as u64;
    let mut params: [*mut core::ffi::c_void; 3] = [
        (&raw mut p_x).cast(),
        (&raw mut p_part).cast(),
        (&raw mut nn).cast(),
    ];
    stream
        .launch(&kernel, [grid, 1, 1], [BLOCK, 1, 1], &mut params)
        .map_err(|e| format!("launch: {e:?}"))?;
    stream.synchronize().map_err(|e| format!("sync: {e:?}"))?;

    let mut partials = vec![0f32; nblocks];
    dpart
        .copy_to_host(&mut partials)
        .map_err(|e| format!("D2H partials: {e:?}"))?;
    // 跨 block 合并(f64 累加,与 host 参考口径一致),结果写回 1 元素标量张量。
    let sum = partials.iter().map(|&v| v as f64).sum::<f64>() as f32;
    // SAFETY: out 为 PyTorch CUDA 1 元素 f32 标量张量设备指针,同一 primary context
    // 内有效可写、借用期内不释放(RXS-0124)。
    let mut d_out = unsafe { ctx.from_device_ptr::<f32>(out, 1) };
    d_out
        .copy_from_host(&[sum])
        .map_err(|e| format!("H2D scalar: {e:?}"))
}

fn run_gemm(c: u64, a: u64, b: u64, m: usize, n: usize, k: usize) -> Result<(), String> {
    if kernels::GEMM_TILE_PTX.trim().is_empty() || kernels::GEMM_TILE_KERNEL.is_empty() {
        return Err("no embedded gemm_tile PTX".to_owned());
    }
    let ctx = Context::from_primary(0).map_err(|e| format!("from_primary: {e:?}"))?;
    // SAFETY: a/b/c 为 PyTorch CUDA 张量设备指针(行主序 f32:a=m*k,b=k*n,c=m*n),
    // 同一 primary context 内有效、借用期内不释放(RXS-0124)。
    let d_a = unsafe { ctx.from_device_ptr::<f32>(a, m * k) };
    // SAFETY: 同上(b 输入张量,k*n 个 f32)。
    let d_b = unsafe { ctx.from_device_ptr::<f32>(b, k * n) };
    // SAFETY: 同上(c 输出张量,m*n 个 f32)。
    let d_c = unsafe { ctx.from_device_ptr::<f32>(c, m * n) };
    let module = ctx
        .load_module(kernels::GEMM_TILE_PTX)
        .map_err(|e| format!("load_module: {e:?}"))?;
    let kernel = module
        .function(kernels::GEMM_TILE_KERNEL)
        .map_err(|e| format!("function: {e:?}"))?;
    let stream = ctx.create_stream().map_err(|e| format!("stream: {e:?}"))?;

    let mut p_a = d_a.device_ptr();
    let mut p_b = d_b.device_ptr();
    let mut p_c = d_c.device_ptr();
    let mut mm: u64 = m as u64;
    let mut nn: u64 = n as u64;
    let mut kk: u64 = k as u64;
    let mut params: [*mut core::ffi::c_void; 6] = [
        (&raw mut p_a).cast(),
        (&raw mut p_b).cast(),
        (&raw mut p_c).cast(),
        (&raw mut mm).cast(),
        (&raw mut nn).cast(),
        (&raw mut kk).cast(),
    ];
    let gx = (n as u32).div_ceil(TILE);
    let gy = (m as u32).div_ceil(TILE);
    stream
        .launch(&kernel, [gx, gy, 1], [TILE, TILE, 1], &mut params)
        .map_err(|e| format!("launch: {e:?}"))?;
    stream.synchronize().map_err(|e| format!("sync: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0123, RXS-0124
    // 双协议(CAI v3 / DLPack)设备指针合法性:空指针(协议抽取失败 / 未初始化)→
    // RX7014,先于任何 GPU 调用(借用外部设备指针前置校验,所有权留外部 deleter)。
    #[test]
    fn null_device_ptr_rejected() {
        assert_eq!(saxpy(0, 0, 0, 1.0, 16), RX_INTEROP_INVALID_DEVICE_PTR);
        assert_eq!(reduce(0, 0, 16), RX_INTEROP_INVALID_DEVICE_PTR);
        assert_eq!(gemm(0, 0, 0, 4, 4, 4), RX_INTEROP_INVALID_DEVICE_PTR);
    }

    //@ spec: RXS-0123
    // 形状合法性:维度为 0 → RX7015,先于任何 GPU 调用(指针非 0 占位,仅校验维度)。
    #[test]
    fn zero_dim_rejected() {
        assert_eq!(saxpy(16, 32, 48, 1.0, 0), RX_INTEROP_SHAPE_MISMATCH);
        assert_eq!(reduce(16, 32, 0), RX_INTEROP_SHAPE_MISMATCH);
        assert_eq!(gemm(16, 32, 48, 0, 4, 4), RX_INTEROP_SHAPE_MISMATCH);
        assert_eq!(gemm(16, 32, 48, 4, 0, 4), RX_INTEROP_SHAPE_MISMATCH);
        assert_eq!(gemm(16, 32, 48, 4, 4, 0), RX_INTEROP_SHAPE_MISMATCH);
    }

    //@ spec: RXS-0125
    // C ABI 边界:ffi 层薄包安全 API,返回码语义一致(段位 RX7013~RX7015 + 0/运行时)。
    #[test]
    fn ffi_thin_wrapper_codes_consistent() {
        // 校验在 GPU 之前,故 host 上可确定性核对(对齐 safe API)。
        assert_eq!(
            ffi::rurix_uc01_saxpy(0, 0, 0, 1.0, 16),
            RX_INTEROP_INVALID_DEVICE_PTR
        );
        assert_eq!(ffi::rurix_uc01_reduce(16, 32, 0), RX_INTEROP_SHAPE_MISMATCH);
        assert_eq!(
            ffi::rurix_uc01_gemm(16, 32, 48, 4, 4, 0),
            RX_INTEROP_SHAPE_MISMATCH
        );
        // 段位常量含义冻结(07 §5):互操作诊断 7013~7015。
        assert_eq!(RX_INTEROP_UNSUPPORTED_PROTOCOL, 7013);
        assert_eq!(RX_INTEROP_INVALID_DEVICE_PTR, 7014);
        assert_eq!(RX_INTEROP_SHAPE_MISMATCH, 7015);
    }

    //@ spec: RXS-0122
    // rx build --emit=pyd PYD 产出约定:nanobind + scikit-build-core 工程模板齐备
    // (pyproject.toml / CMakeLists.txt / binding.cpp),供 rx build 编排打包链接
    // rurix-interop staticlib;算子内省覆盖 UC-01 三算子(SAXPY/Reduction/GEMM)。
    #[test]
    fn pyd_project_template_present() {
        let pyd = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pyd");
        assert!(
            pyd.join("pyproject.toml").is_file(),
            "PYD 工程缺 pyproject.toml(scikit-build-core 后端)"
        );
        assert!(
            pyd.join("CMakeLists.txt").is_file(),
            "PYD 工程缺 CMakeLists.txt(nanobind 模块 + 链接 rurix-interop)"
        );
        assert!(
            pyd.join("src").join("binding.cpp").is_file(),
            "PYD 工程缺 src/binding.cpp(nanobind 绑定)"
        );
        // C ABI 导出符号(RXS-0125)在 ffi 模块齐备(UC-01 三算子)。
        let _ = ffi::rurix_uc01_saxpy;
        let _ = ffi::rurix_uc01_reduce;
        let _ = ffi::rurix_uc01_gemm;
    }
}
