//! rurix-d3d12 构建脚本（RFC-0001 §4.2）。
//!
//! **默认（无 real-shim feature）**：no-op —— 不编译任何 C++，crate 退化为纯 Rust
//! stub，无 SDK 环境亦可 `cargo build --workspace` 绿（常驻回归网）。
//!
//! **real-shim feature**：经 `cc` 把 `shim/rx_d3d12_shim.cpp`（D3D12/DXGI COM +
//! 固定 present pass）编译为静态库并链接 Windows SDK D3D12 组件。需 MSVC + Windows
//! SDK（D3D12 头/库 + d3dcompiler）。HLSL `shim/present.hlsl` 由 shim 运行期/构建期
//! 经 d3dcompiler 编译（present pass 私有资产，非 Rurix shader codegen）。

fn main() {
    // build.rs 自身按 package feature 编译（CARGO_FEATURE_* 环境变量为权威判定）。
    if std::env::var_os("CARGO_FEATURE_REAL_SHIM").is_none() {
        return; // stub：不触 C++ / 不链接 D3D12。
    }

    println!("cargo:rerun-if-changed=shim/rx_d3d12_shim.cpp");
    println!("cargo:rerun-if-changed=shim/present.hlsl");

    cc::Build::new()
        .cpp(true)
        .file("shim/rx_d3d12_shim.cpp")
        .std("c++17")
        .compile("rx_d3d12_shim");

    // Windows SDK D3D12 系统组件（不受 NVIDIA 再分发约束，G1_CONTRACT §5）。
    for lib in ["d3d12", "dxgi", "d3dcompiler", "user32", "gdi32"] {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
}
