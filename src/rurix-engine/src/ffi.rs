//! C ABI 导出层（G1.3，RXS-0149 / MR-0002）：自建 C++/D3D12 harness 经
//! [`include/rurix_engine.h`](../include/rurix_engine.h) + import lib 链接本 `cdylib`，在最小
//! render-graph 上下文中承担 compute pass。每个 `extern "C"` 入口为 [`rurix_interop`] 既有
//! safe API（RXS-0125，**语义 0-byte**）的薄前向：接受由宿主提供的设备指针（[`u64`]，与 D3D12
//! 同 adapter / 同 device primary context，复用 G1.1 interop 路径）+ 维度（[`u64`]）按值，返回
//! [`i32`] 错误码（`0` = 成功；互操作诊断段位 RX7013~RX7015；负 = 运行时/驱动失败，07 §5）。
//!
//! C ABI 边界（Windows x64 唯一 ABI，D-113）：`extern "C"` + 标量按值，无裸指针解引用（设备
//! 指针为不透明 `u64` 地址，仅前向给 safe API）；故本层**无 `unsafe` 块**，unsafe 仅在
//! [`rurix_interop`] safe API 内借用外部设备指针处（`// SAFETY:` + 注册）。导出属性
//! `#[unsafe(no_mangle)]` 经裁决最小开（unsafe-audit/rurix-engine.md，U21）。
//!
//! **不实现 D-113 `#[export(c)]` 编译器 codegen**（defer，RD-009）：以手写 `extern "C"` 导出 +
//! 与本组导出 **1:1** 的随附头文件兑现；头↔导出一致性由 `crate::tests::c_abi_header_matches_exports`
//! 守卫（漂移即红）。

use crate::RX_ENGINE_ABI_VERSION;

/// C ABI：引擎集成 ABI 版本握手（宿主链接前校核，对齐 RFC-0001 §4.2.1 ABI 版本约定）。
#[unsafe(no_mangle)]
pub extern "C" fn rurix_engine_abi_version() -> u32 {
    RX_ENGINE_ABI_VERSION
}

/// C ABI：SAXPY compute pass `out = a*x + y`（`n` 个 `f32` 设备指针；前向
/// [`rurix_interop::saxpy`]，RXS-0149 复用 RXS-0125）。返回 `0` / RX7014 / RX7015 / 负=运行时。
#[unsafe(no_mangle)]
pub extern "C" fn rurix_engine_compute_saxpy(out: u64, x: u64, y: u64, a: f32, n: u64) -> i32 {
    rurix_interop::saxpy(out, x, y, a, n as usize)
}

/// C ABI：Reduction compute pass `out[0] = Σ x`（`x` n 个 f32，`out` 1 元素；前向
/// [`rurix_interop::reduce`]，RXS-0149 复用 RXS-0125）。返回 `0` / RX7014 / RX7015 / 负=运行时。
#[unsafe(no_mangle)]
pub extern "C" fn rurix_engine_compute_reduce(out: u64, x: u64, n: u64) -> i32 {
    rurix_interop::reduce(out, x, n as usize)
}
