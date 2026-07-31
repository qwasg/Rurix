//! G6.2 PR-A:vendor 内联构建 JoltC + Jolt(RFC-0017 §4.C1 I-1 裁决,定案登记 VENDOR.md §2)。
//!
//! 经 `cmake` crate 把 `vendor/JoltC`(内含 `vendor/JoltC/JoltPhysics`)编为静态库
//! `joltc` + `Jolt` 并链入;上游 commit pin 与 cmake 配置理由逐条见 VENDOR.md。
//! 构建画像:x86_64-pc-windows-msvc + VS2022 Community + cmake 4.3;C++ 侧固定 Release。

use std::path::Path;

fn main() {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/JoltC");
    assert!(
        vendor.join("CMakeLists.txt").is_file(),
        "vendor/JoltC 缺失(见 src/rurix-physics-sys/VENDOR.md §1 vendor 布局)"
    );

    let dst = cmake::Config::new(&vendor)
        // 上游默认 ON(/MT)与 Rust MSVC 默认动态 CRT(/MD)混链必 LNK2038,强制 OFF。
        .define("USE_STATIC_MSVC_RUNTIME_LIBRARY", "OFF")
        // 上游默认 ON(/GL+/LTCG),构建时间数倍膨胀;底座首版关闭(性能不进硬门,P-09)。
        .define("INTERPROCEDURAL_OPTIMIZATION", "OFF")
        // cmake 4.x 移除 < 3.5 兼容的防御钉(JoltC 要 3.16 / Jolt 要 3.20,当前不触发)。
        .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
        // 单精度 + object layer 16 位(与 JoltC JPC_OBJECT_LAYER_BITS=16 一致,VENDOR.md §2)。
        .define("DOUBLE_PRECISION", "OFF")
        .define("OBJECT_LAYER_BITS", "16")
        // 确定性口径 (a):同二进制同平台逐位(§4.0-4);可选 (b) 本切片不启用。
        .define("CROSS_PLATFORM_DETERMINISTIC", "OFF")
        // Jolt Debug 配置对单测不可用地慢;Release CRT(/MD)与 Rust debug 二进制兼容。
        .profile("Release")
        .build();

    // cmake crate 默认跑 `install` target:产物 <dst>/lib/{joltc.lib, Jolt.lib}。
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=joltc");
    println!("cargo:rustc-link-lib=static=Jolt");
    // vendor 树为冻结 pin(VENDOR.md),只盯 build.rs 自身,避免整包变更触发 C++ 重编。
    println!("cargo:rerun-if-changed=build.rs");
}
