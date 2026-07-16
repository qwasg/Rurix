//! rurix-android-present 构建脚本(mb1 W7,G-MB1-7 Phase B)。
//!
//! **仅 android target**:链接 NativeActivity glue 所需系统库 + 16KB page 对齐旗标(荣耀
//! BKQ-AN10 / 现代 Android 要求共享库 16KB 对齐);`__android_log_write`(rurix-rt android
//! messenger 也引用)、`ANativeWindow_acquire/release`、NativeActivity ABI 由这些库提供。
//! **桌面(非 android)no-op**:整 crate `#![cfg(target_os="android")]` 为空 lib,无需链接。

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "android" {
        // 桌面:空 lib,不发链接指令(零回归)。
        return;
    }
    // liblog:__android_log_write(app RESULT + validation VVL logcat)。
    println!("cargo:rustc-link-lib=dylib=log");
    // libandroid:NativeActivity 运行时。
    println!("cargo:rustc-link-lib=dylib=android");
    // libnativewindow:ANativeWindow_acquire/release(现代 NDK 从 libandroid 迁出)。
    println!("cargo:rustc-link-lib=dylib=nativewindow");
    // 16KB page 对齐(荣耀/现代 Android;.so 段对齐;zipalign -P 16 在打包侧配合)。
    println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
    println!("cargo:rustc-link-arg=-Wl,-z,common-page-size=16384");
}
