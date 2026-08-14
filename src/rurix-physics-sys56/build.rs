//! G9.6 M125:vendor 内联构建 JoltC(5.6 线)+ Jolt v5.6.0(RXS-0377 七步程序第②步;
//! 沿 5.3 线 RFC-0017 §4.C1 I-1 裁决同构,定案登记 VENDOR56.md §2)。
//!
//! 经 `cmake` crate 把 `vendor/JoltC`(内含 `vendor/JoltC/JoltPhysics` = Jolt v5.6.0)
//! 编为静态库 `joltc` + `Jolt` 并链入;上游 commit pin 与 cmake 配置理由逐条见
//! VENDOR56.md。**符号隔离**:vendor 源码经机械重命名(`JPC_`→`JPC56_` /
//! `namespace JPH`→`namespace JPH56`),与 5.3 基线静态库同进程并存零符号冲突。
//! 构建画像:x86_64-pc-windows-msvc + VS2022 Community + cmake 4.3;C++ 侧固定 Release。
//!
//! **GPU compute 只评估不接权威**(RXS-0377 L4;GPU 主刚体禁止线 0-byte):
//! Jolt 5.6 新增 GPU compute shader 接口在本 vendor 构建中编译期整体关闭
//! (`JPH_USE_DX12/VK/MTL/CPU_COMPUTE=OFF`)——接口不可达为结构性断言,
//! 评估报告留档(VENDOR56.md §4),接入须 RD-043 + 矩阵 §12 + 独立 Full RFC。

use std::path::Path;

fn main() {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/JoltC");
    assert!(
        vendor.join("CMakeLists.txt").is_file(),
        "vendor/JoltC 缺失(见 src/rurix-physics-sys56/VENDOR56.md §1 vendor 布局)"
    );

    let dst = cmake::Config::new(&vendor)
        // 上游默认 ON(/MT)与 Rust MSVC 默认动态 CRT(/MD)混链必 LNK2038,强制 OFF。
        .define("USE_STATIC_MSVC_RUNTIME_LIBRARY", "OFF")
        // 上游默认 ON(/GL+/LTCG),构建时间数倍膨胀;评估臂同样关闭(性能不进硬门,P-09)。
        .define("INTERPROCEDURAL_OPTIMIZATION", "OFF")
        // cmake 4.x 移除 < 3.5 兼容的防御钉(JoltC 要 3.16 / Jolt 5.6 要 3.20,当前不触发)。
        .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
        // 单精度 + object layer 16 位(与 5.3 线画像逐项一致,A/B 同画像前提)。
        .define("DOUBLE_PRECISION", "OFF")
        .define("OBJECT_LAYER_BITS", "16")
        // 确定性口径 (a):同二进制同平台逐位(与 5.3 线同一口径;可选 (b) 不启用)。
        .define("CROSS_PLATFORM_DETERMINISTIC", "OFF")
        // GPU compute 只评估不接权威(RXS-0377 L4):四开关 OFF 编译期排除
        // Jolt/Compute/** 与 GPU 毛发(Jolt/Shaders/**)——接口不可达结构性断言。
        .define("JPH_USE_DX12", "OFF")
        .define("JPH_USE_VK", "OFF")
        .define("JPH_USE_MTL", "OFF")
        .define("JPH_USE_CPU_COMPUTE", "OFF")
        // Jolt Debug 配置对单测不可用地慢;Release CRT(/MD)与 Rust debug 二进制兼容。
        .profile("Release")
        .build();

    // cmake crate 默认跑 `install` target:产物 <dst>/lib/{joltc.lib, Jolt.lib}。
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=joltc");
    println!("cargo:rustc-link-lib=static=Jolt");
    // vendor 树为冻结 pin(VENDOR56.md),只盯 build.rs 自身,避免整包变更触发 C++ 重编。
    println!("cargo:rerun-if-changed=build.rs");
}
