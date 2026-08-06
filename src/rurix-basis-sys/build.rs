//! G8.3 M83:经 `cc` 编译过渡纹理 codec C++ shim(设计案 §3.6:不用 cmake)。
//! 完整 `basis_universal` vendor 待合入时,本清单改为显式 .cpp 文件列表 + 同 flags 纪律。

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let shim_dir = manifest.join("vendor/rurix_basis_shim");
    let cpp = shim_dir.join("rurix_basis_shim.cpp");
    let hdr = shim_dir.join("rurix_basis_shim.h");
    assert!(
        cpp.is_file(),
        "vendor shim 缺失: {}(见 VENDOR.md)",
        cpp.display()
    );

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", cpp.display());
    println!("cargo:rerun-if-changed={}", hdr.display());

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file(&cpp)
        .include(&shim_dir)
        // 确定性钳制:禁 RDO/多线程路径宏位(shim 内亦硬编码 threads=1)。
        .define("RURIX_BASIS_THREADS", Some("1"))
        .define("RURIX_BASIS_NO_ZSTD", None)
        .warnings(false)
        .compile("rurix_basis_shim");
}
