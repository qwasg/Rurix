//! C ABI 导出层(M8.2,RXS-0128):`ci/cublas_binding_smoke.py` 经 ctypes 加载本 cdylib,
//! 以 torch CUDA 张量设备指针**零拷贝**调用。每个 `extern "C"` 入口为 [`crate`] 高层
//! API 的薄包,接受设备指针([`u64`])+ 维度([`u64`]),返回 [`i32`] 错误码(0 = 成功;
//! cublas 诊断段位 RX7016~RX7019,07 §5)。
//!
//! C ABI 边界(Windows x64 唯一 ABI,D-113):`extern "C"` + 标量按值传参,无裸指针
//! 解引用(设备指针为不透明 `u64` 地址,仅前向给高层 safe API);故本层无 `unsafe` 块,
//! unsafe 仅在 [`sys`](crate::sys) cublas FFI 调用处(`// SAFETY:` + unsafe-audit 注册)。

use crate::{gemm, gemv};

/// C ABI:GEMM `C[M,N] = A[M,K]·B[K,N]`(行主序 f32 设备指针;RXS-0128)。
#[unsafe(no_mangle)]
pub extern "C" fn rurix_cublas_gemm(c: u64, a: u64, b: u64, m: u64, n: u64, k: u64) -> i32 {
    gemm(c, a, b, m as usize, n as usize, k as usize)
}

/// C ABI:GEMV `y[M] = A[M,N]·x[N]`(行主序 f32 设备指针,`m` = A 行数 = y 长度,
/// `n` = A 列数 = x 长度;RXS-0128)。
#[unsafe(no_mangle)]
pub extern "C" fn rurix_cublas_gemv(y: u64, a: u64, x: u64, m: u64, n: u64) -> i32 {
    gemv(y, a, x, m as usize, n as usize)
}
