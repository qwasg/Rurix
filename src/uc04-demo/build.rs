//! uc04-demo 构建脚本(G2.4 / RFC-0006;选项 B 不采样 G-buffer 的最小多 pass deferred)。
//!
//! **默认(无 real-shim feature)**:no-op —— 不编译任何 C++,crate 退化为纯 Rust host
//! 装配/编排模型 + device stub,无 SDK 环境亦可 `cargo build --workspace` 绿(常驻回归网)。
//!
//! **real-shim feature**:经 `cc` 把 `shim/uc04_offscreen.cpp`(D3D12 离屏两 pass deferred
//! draw + readback,消费 Rurix 图形=B DXIL + RFC-0005 RTS0)编译为静态库并链接 Windows SDK
//! D3D12 组件。需 MSVC + Windows SDK(D3D12 头/库)。对齐 src/rurix-d3d12/build.rs 先例。

fn main() {
    // build.rs 自身按 package feature 编译(CARGO_FEATURE_* 环境变量为权威判定)。
    if std::env::var_os("CARGO_FEATURE_REAL_SHIM").is_none() {
        return; // stub:不触 C++ / 不链接 D3D12。
    }

    println!("cargo:rerun-if-changed=shim/uc04_offscreen.cpp");

    cc::Build::new()
        .cpp(true)
        .file("shim/uc04_offscreen.cpp")
        .std("c++17")
        .compile("uc04_offscreen_shim");

    // Windows SDK D3D12 系统组件(不受 NVIDIA 再分发约束,G2_CONTRACT §5)。
    for lib in ["d3d12", "dxgi", "user32"] {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
}
