//! rurix-godot build script (GRX-009 segment 4d).
//!
//! **Default (no `d3d12-recording-shim` feature)**: no-op — no C++ is compiled
//! and no D3D12 component is linked, so `cargo build/test -p rurix-godot`
//! needs no Windows SDK / D3D12 and keeps the fallback-only bridge behaviour.
//!
//! **`d3d12-recording-shim` feature**: compile `shim/rxgd_luminance_record.cpp`
//! (a Windows-only C++ D3D12 compute-dispatch recording shim) via `cc` and link
//! the Windows SDK D3D12 components. Mirrors `src/uc04-demo/build.rs`.
//!
//! DXIL signing: the compiler-emitted DXIL container needs the DXIL validator
//! hash to create a compute PSO on a non-Developer-Mode device. When
//! `RURIX_DXC_DIR` (or `RURIX_DXC_NEW_DIR`) points at a signed DXC pin whose
//! `dxcapi.h` can be found, `RXGD_HAVE_DXCAPI` is defined so the shim can sign
//! the in-memory DXIL copy (never the tracked artifact bytes) at runtime via
//! `dxil.dll`. Without it, the shim compiles but reports signing unavailable,
//! so recording only succeeds where signing is actually possible — it never
//! fakes a dispatch.

fn main() {
    // build.rs is itself compiled for every build; the feature is authoritative
    // via the CARGO_FEATURE_* environment variable.
    if std::env::var_os("CARGO_FEATURE_D3D12_RECORDING_SHIM").is_none() {
        return; // stub: touch no C++, link no D3D12.
    }

    println!("cargo:rerun-if-changed=shim/rxgd_luminance_record.cpp");
    println!("cargo:rerun-if-env-changed=RURIX_DXC_DIR");
    println!("cargo:rerun-if-env-changed=RURIX_DXC_NEW_DIR");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("shim/rxgd_luminance_record.cpp")
        .std("c++17");

    if let Some(include_dir) = dxcapi_include_dir() {
        build.include(&include_dir);
        build.define("RXGD_HAVE_DXCAPI", None);
    }

    build.compile("rxgd_luminance_record_shim");

    // Windows SDK D3D12 system components (not subject to NVIDIA redistribution
    // constraints; same set uc04-demo links).
    for lib in ["d3d12", "dxgi"] {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
}

/// Locate the directory containing `dxcapi.h` near a signed DXC pin so the shim
/// can sign the in-memory DXIL container. Mirrors the resolution used by
/// `ci/grx009_luminance_d3d12_dispatch_smoke.py::locate_dxcapi_include`.
fn dxcapi_include_dir() -> Option<std::path::PathBuf> {
    for key in ["RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"] {
        let Some(raw) = std::env::var_os(key) else {
            continue;
        };
        let dxc_dir = std::path::PathBuf::from(raw);
        let mut bases: Vec<std::path::PathBuf> = vec![dxc_dir.clone()];
        bases.extend(dxc_dir.ancestors().map(std::path::Path::to_path_buf));
        for base in bases {
            for name in ["inc", "include"] {
                let candidate = base.join(name).join("dxcapi.h");
                if candidate.is_file() {
                    return candidate.parent().map(std::path::Path::to_path_buf);
                }
            }
        }
    }
    None
}
