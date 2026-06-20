//! `rurix-engine` — 引擎集成 C ABI 边界（G1.3，D-G1-3 / G-G1-3，UC-05 前奏；spec/engine_integration.md
//! RXS-0149 / MR-0002）。
//!
//! 首个引擎集成：把承担 **compute pass** 的 C ABI 编为 **`cdylib` DLL**（`rurix_engine.dll` +
//! import lib，`rurix-cublas` cdylib 先例），供自建最小 **C++/D3D12 渲染 harness** 经
//! [`include/rurix_engine.h`](../include/rurix_engine.h) + import lib 链接，在最小 render-graph
//! 上下文中承担 ≥1 个 compute pass（06 §8.3 / 02 §U5）。
//!
//! **复用**（MR-0002，最大化复用、不重新发明 C ABI）：compute pass 经 [`rurix_interop`] 既有
//! safe API（`saxpy`/`reduce`，RXS-0125，**语义 0-byte**）前向，复用其经 build.rs 嵌入的 M5 自研
//! kernel PTX（PTX-only）。**不实现 D-113 `#[export(c)]` 编译器 codegen + 内建头文件生成**（defer，
//! RD-009）——以与导出 ABI **1:1** 的随附头文件兑现头文件单一事实源方向，头↔ABI 一致性由
//! [`tests::c_abi_header_matches_exports`] + CI 步骤 43 host 段守卫（漂移即红）。
//!
//! **分层**（RXS-0149 / MR-0002 §7）：本 crate 经裁决最小开 `unsafe_code`（仅 C ABI 导出属性
//! `#[unsafe(no_mangle)] extern "C"`，注册见 `unsafe-audit/rurix-engine.md` U21）；[`ffi`] 导出层
//! 前向 [`rurix_interop`] safe API，本层**无 `unsafe` 块**，对上全 safe。
//!
//! **范围红线**：仅承担 compute pass，**不进图形着色阶段 / DXIL 第二后端**（G2，06 §8.2 / D-131）；
//! 永不 Python 原生嵌入（红线 1，SG-008，仅 C ABI 通道）。

pub mod ffi;

pub use ffi::*;

/// 引擎集成 C ABI 版本（与 C++ `include/rurix_engine.h` 的 `RURIX_ENGINE_ABI_VERSION` 一致）。
pub const RX_ENGINE_ABI_VERSION: u32 = 1;

/// 随附头文件（`include/rurix_engine.h`）声明的 C ABI 导出符号集 —— **单一事实源**，与头文件
/// 逐一对应（RXS-0149：导出符号集 ↔ 头文件声明 1:1，无悬空声明 / 无未声明导出）。
/// 由 [`tests::c_abi_header_matches_exports`] 对头文件实际声明集断言一致（漂移即红）。
pub const EXPORTED_C_ABI: &[&str] = &[
    "rurix_engine_abi_version",
    "rurix_engine_compute_saxpy",
    "rurix_engine_compute_reduce",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// 从随附头文件提取声明的 C ABI 导出函数名（出现在 `(` 之前的 `rurix_engine_*` 标识符）。
    fn header_declared_exports(header: &str) -> Vec<String> {
        const PREFIX: &str = "rurix_engine_";
        let mut names = Vec::new();
        for line in header.lines() {
            let line = line.trim_start();
            // 跳过注释 / 宏 / 非声明行；仅取「<ident>(」形态的函数声明。
            if line.starts_with("//") || line.starts_with('*') || line.starts_with('#') {
                continue;
            }
            let mut rest = line;
            while let Some(pos) = rest.find(PREFIX) {
                let tail = &rest[pos..];
                let end = tail
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(tail.len());
                let ident = &tail[..end];
                // 函数声明：标识符后(去空白)紧跟 '('。
                if tail[end..].trim_start().starts_with('(') {
                    let name = ident.to_owned();
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
                rest = &tail[end..];
            }
        }
        names
    }

    //@ spec: RXS-0149
    // 引擎集成 DLL 打包与 C ABI 头文件对应:随附头文件(include/rurix_engine.h)声明的导出符号集
    // 与 crate 权威导出集(EXPORTED_C_ABI,单一事实源)**逐一对应**——无悬空声明 / 无未声明导出
    // (头与导出漂移即红;反 YAML-only,CI 步骤 43 host 段闸门)。同时编译期引用每个导出确保其
    // 以 C ABI 签名存在(`#[unsafe(no_mangle)] extern "C"`)。
    #[test]
    fn c_abi_header_matches_exports() {
        let header_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("include/rurix_engine.h");
        let header = std::fs::read_to_string(&header_path)
            .unwrap_or_else(|e| panic!("缺随附头文件 {}: {e}", header_path.display()));

        let mut declared = header_declared_exports(&header);
        declared.sort();
        let mut expected: Vec<String> = EXPORTED_C_ABI.iter().map(|s| (*s).to_owned()).collect();
        expected.sort();
        assert_eq!(
            declared, expected,
            "头文件声明集与 EXPORTED_C_ABI 漂移(RXS-0149:头↔导出须 1:1);\n  头声明={declared:?}\n  导出集={expected:?}"
        );

        // 编译期引用每个导出,确保以 C ABI 签名存在(签名漂移即编译失败)。
        let _: extern "C" fn() -> u32 = ffi::rurix_engine_abi_version;
        let _: extern "C" fn(u64, u64, u64, f32, u64) -> i32 = ffi::rurix_engine_compute_saxpy;
        let _: extern "C" fn(u64, u64, u64) -> i32 = ffi::rurix_engine_compute_reduce;

        assert_eq!(ffi::rurix_engine_abi_version(), RX_ENGINE_ABI_VERSION);
    }

    //@ spec: RXS-0149
    // compute pass C ABI 复用 rurix-interop 既有诊断段位(RXS-0125 RX7013~7015,语义 0-byte):
    // 设备指针非法 / 维度为 0 在 GPU 之前确定性拦截(host 上可核对,不依赖 device)。
    #[test]
    fn compute_pass_reuses_interop_diagnostics() {
        use rurix_interop::{RX_INTEROP_INVALID_DEVICE_PTR, RX_INTEROP_SHAPE_MISMATCH};
        assert_eq!(
            ffi::rurix_engine_compute_saxpy(0, 0, 0, 1.0, 16),
            RX_INTEROP_INVALID_DEVICE_PTR
        );
        assert_eq!(
            ffi::rurix_engine_compute_reduce(16, 32, 0),
            RX_INTEROP_SHAPE_MISMATCH
        );
    }
}
