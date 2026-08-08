//! G8.3 M83:经 `cc` 编译真实 `basis_universal`(BinomialLLC 1.16.4)+ 薄 C 包装
//! (设计案 §3.6:**禁 cmake**)。编译单元 = 显式 .cpp 清单(见 VENDOR.md / vendor_manifest.json)。
//!
//! 确定性钳制(RXS-0334):线程恒 1、禁 zstd supercompression、禁 OpenCL、禁 SSE 特化路径
//! (跨机同字节),固定算法序由 wrap 侧参数锁定。

use std::env;
use std::path::PathBuf;

/// 上游编译单元显式清单(对齐 vendor_manifest.json `compile_units`,剔除
/// `basisu_tool.cpp`(CLI main)/`zstd/`(禁 supercompression)/OpenCL kernel 面)。
const ENCODER_CPP: &[&str] = &[
    "basisu_backend.cpp",
    "basisu_basis_file.cpp",
    "basisu_bc7enc.cpp",
    "basisu_comp.cpp",
    "basisu_enc.cpp",
    "basisu_etc.cpp",
    "basisu_frontend.cpp",
    "basisu_gpu_texture.cpp",
    "basisu_kernels_sse.cpp",
    "basisu_opencl.cpp",
    "basisu_pvrtc1_4.cpp",
    "basisu_resample_filters.cpp",
    "basisu_resampler.cpp",
    "basisu_ssim.cpp",
    "basisu_uastc_enc.cpp",
    // basisu_enc.cpp 的 load_jpg/load_png 引用这两个 reader 的符号(即便本 crate
    // 不走文件读取路径,链接期仍须满足)。保留 = 与 vendor_manifest.json
    // `compile_units` 清单一致。
    "jpgd.cpp",
    "pvpngreader.cpp",
];

const TRANSCODER_CPP: &[&str] = &["basisu_transcoder.cpp"];

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let vendor = manifest.join("vendor/basis_universal");
    let encoder = vendor.join("encoder");
    let transcoder = vendor.join("transcoder");
    let ffi_dir = manifest.join("ffi");
    let wrap_cpp = ffi_dir.join("rurix_basis_wrap.cpp");
    let wrap_h = ffi_dir.join("rurix_basis_wrap.h");

    assert!(
        vendor.join("vendor_manifest.json").is_file(),
        "vendor 快照缺失: {}(见 VENDOR.md;跑 py -3 ci/vendor_basis_universal.py)",
        vendor.display()
    );
    assert!(wrap_cpp.is_file(), "FFI wrap 缺失: {}", wrap_cpp.display());

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", wrap_cpp.display());
    println!("cargo:rerun-if-changed={}", wrap_h.display());

    let mut b = cc::Build::new();
    b.cpp(true).std("c++17");
    b.include(&encoder).include(&transcoder).include(&ffi_dir);

    // 确定性钳制(RXS-0334):
    //  - BASISU_SUPPORT_SSE=0   → 禁 SSE 特化码路(跨 CPU 同字节)
    //  - BASISU_SUPPORT_OPENCL=0→ 禁 GPU/OpenCL 编码路径
    //  - BASISD_SUPPORT_KTX2=1  → KTX2 容器支持(基础设施,非 supercompression)
    //  - BASISD_SUPPORT_KTX2_ZSTD=0 → **禁 zstd supercompression**(同时移除 zstd/ 依赖)
    b.define("BASISU_SUPPORT_SSE", Some("0"))
        .define("BASISU_SUPPORT_OPENCL", Some("0"))
        .define("BASISD_SUPPORT_KTX2", Some("1"))
        .define("BASISD_SUPPORT_KTX2_ZSTD", Some("0"))
        .define("RURIX_BASIS_THREADS", Some("1"));

    for f in ENCODER_CPP {
        let p = encoder.join(f);
        assert!(p.is_file(), "上游编译单元缺失: {}", p.display());
        println!("cargo:rerun-if-changed={}", p.display());
        b.file(&p);
    }
    for f in TRANSCODER_CPP {
        let p = transcoder.join(f);
        assert!(p.is_file(), "上游编译单元缺失: {}", p.display());
        println!("cargo:rerun-if-changed={}", p.display());
        b.file(&p);
    }
    b.file(&wrap_cpp);

    // 上游 warning 噪声与本仓纪律无关(vendor 快照不改源)。
    b.warnings(false).flag_if_supported("-Wno-everything");
    b.compile("rurix_basis_wrap");

    // MSVC 以外需显式链 C++ 标准库(cc 已处理大部分场景,保留 pthread 面)。
    if !cfg!(target_env = "msvc") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
}
