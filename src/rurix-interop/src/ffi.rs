//! C ABI 导出层(M8.1,RXS-0125):nanobind 绑定经 scikit-build-core 链接本 staticlib
//! 产 PYD。每个 `extern "C"` 入口为 [`crate`] safe API 的薄包,接受由 nanobind 经
//! `__cuda_array_interface__` v3 / DLPack 双协议**零拷贝**抽取的 PyTorch CUDA 张量
//! 设备指针([`u64`])+ 维度([`u64`]),返回 [`i32`] 错误码(0 = 成功;互操作诊断
//! 段位 RX7013~RX7015;负 = 运行时/驱动失败,07 §5)。
//!
//! C ABI 边界(Windows x64 唯一 ABI,D-113):`extern "C"` + 标量按值传参,无裸指针
//! 解引用(设备指针为不透明 `u64` 地址,仅前向给 safe API);故本层无 `unsafe` 块,
//! unsafe 仅在 safe API 内借用外部设备指针处(`// SAFETY:` + unsafe-audit 注册)。

use crate::{gemm, reduce, saxpy};

/// C ABI:SAXPY 算子替换 `out = a*x + y`(`n` 个 `f32` 设备指针;RXS-0123/0124/0125)。
#[unsafe(no_mangle)]
pub extern "C" fn rurix_uc01_saxpy(out: u64, x: u64, y: u64, a: f32, n: u64) -> i32 {
    saxpy(out, x, y, a, n as usize)
}

/// C ABI:Reduction 算子替换 `out[0] = Σ x`(`x` n 个 f32,`out` 1 元素;RXS-0123/0124/0125)。
#[unsafe(no_mangle)]
pub extern "C" fn rurix_uc01_reduce(out: u64, x: u64, n: u64) -> i32 {
    reduce(out, x, n as usize)
}

/// C ABI:GEMM 算子替换 `C[M,N] = A[M,K]·B[K,N]`(行主序 f32 设备指针;RXS-0123/0124/0125)。
#[unsafe(no_mangle)]
pub extern "C" fn rurix_uc01_gemm(c: u64, a: u64, b: u64, m: u64, n: u64, k: u64) -> i32 {
    gemm(c, a, b, m as usize, n as usize, k as usize)
}
