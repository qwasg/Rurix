//! rxcook — G8.3 资产 cook CLI(设计案 §1.2)。
//! 子命令:`import-gltf`(M81) / `cook-texture`(M83) / `decode-page`(M04) /
//! `verify --double-build`(M79) / `coverage-list`。

use rurix_asset::gltf::{self, validate::ImportOptions};
use rurix_asset::texture::{
    CookProfile, TextureSemantics, cook_texture, decode_ppm_p6, encode_ppm_p6,
    fixture_checker_rgba16, fixture_normal_rgba16,
};
use rurix_asset::canon;
use rurix_asset::verify;
use rurix_geom_pages::{
    decode_disk_page, expand_memory_page, expand_u32_count, expanded_digest, encode_memory_page,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn usage() -> ! {
    eprintln!(
        "usage:\n  rxcook import-gltf <path> [--emit-digest]\n  rxcook coverage-list\n  rxcook cook-texture --fixture checker|normal --out <dir> [--profile win-vulkan-bcn-v1]\n  rxcook cook-texture --input <file.ppm> --out <dir> [--profile ...] [--semantics color|normal|mask]\n  rxcook decode-page --disk <p.rxpd> [--emit-expanded-digest] [--emit-rxpm <path>]\n  rxcook verify --double-build [--workspace <root>] [--scratch <dir>]\n  rxcook canon-check --accept <dir> --reject <dir>"
    );
    std::process::exit(2);
}

fn hex32(d: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn cmd_decode_page(args: Vec<String>) -> ExitCode {
    let mut disk: Option<PathBuf> = None;
    let mut emit_digest = false;
    let mut emit_rxpm: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--disk" => {
                i += 1;
                disk = Some(PathBuf::from(args.get(i).unwrap_or_else(|| usage())));
            }
            "--emit-expanded-digest" | "--emit-digest" => emit_digest = true,
            "--emit-rxpm" => {
                i += 1;
                emit_rxpm = Some(PathBuf::from(args.get(i).unwrap_or_else(|| usage())));
            }
            other => {
                eprintln!("unknown decode-page flag: {other}");
                usage();
            }
        }
        i += 1;
    }
    let disk = disk.unwrap_or_else(|| usage());
    let bytes = fs::read(&disk).unwrap_or_else(|e| {
        eprintln!("read {}: {e}", disk.display());
        std::process::exit(1);
    });
    match decode_disk_page(&bytes) {
        Ok(page) => {
            if let Some(p) = emit_rxpm {
                let img = encode_memory_page(&page);
                if let Err(e) = fs::write(&p, &img) {
                    eprintln!("write rxpm: {e}");
                    return ExitCode::FAILURE;
                }
            }
            if emit_digest {
                let d = expanded_digest(&page);
                let n = expand_u32_count(&page);
                let stream = expand_memory_page(&page);
                println!(
                    "{{\"ok\":true,\"expanded_digest\":\"{}\",\"expanded_u32_count\":{},\"expanded_bytes\":{}}}",
                    hex32(&d),
                    n,
                    stream.len()
                );
            } else {
                println!(
                    "ok clusters={} indices={}",
                    page.clusters.len(),
                    page.indices.len()
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("decode-page failed: {e}");
            println!("{{\"ok\":false,\"error\":\"{e}\"}}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_canon_check(args: Vec<String>) -> ExitCode {
    let mut accept = None;
    let mut reject = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--accept" => {
                i += 1;
                accept = Some(PathBuf::from(args.get(i).unwrap_or_else(|| usage())));
            }
            "--reject" => {
                i += 1;
                reject = Some(PathBuf::from(args.get(i).unwrap_or_else(|| usage())));
            }
            other => {
                eprintln!("unknown canon-check flag: {other}");
                usage();
            }
        }
        i += 1;
    }
    let accept = accept.unwrap_or_else(|| usage());
    let reject = reject.unwrap_or_else(|| usage());
    match canon::check_canon_corpus(&accept, &reject) {
        Ok((a, r)) => {
            println!("canon_accept={a}");
            println!("canon_reject={r}");
            println!("ok=true");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("canon-check failed: {e}");
            println!("ok=false");
            ExitCode::from(1)
        }
    }
}

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // src/rurix-asset → repo root
    p.pop();
    p.pop();
    p
}

fn cmd_verify(args: Vec<String>) -> ExitCode {
    let mut double = false;
    let mut workspace = workspace_root();
    let mut scratch = env::temp_dir().join(format!(
        "rxcook_verify_{}",
        std::process::id()
    ));
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--double-build" => double = true,
            "--workspace" => {
                i += 1;
                workspace = PathBuf::from(args.get(i).unwrap_or_else(|| usage()));
            }
            "--scratch" => {
                i += 1;
                scratch = PathBuf::from(args.get(i).unwrap_or_else(|| usage()));
            }
            other => {
                eprintln!("unknown verify flag: {other}");
                usage();
            }
        }
        i += 1;
    }
    if !double {
        eprintln!("verify requires --double-build");
        usage();
    }
    let _ = fs::create_dir_all(&scratch);
    match verify::verify_double_build(&workspace, &scratch) {
        Ok(r) => {
            print!("{}", r.to_checks_json());
            println!("ok={}", r.all_pass());
            println!("left_manifest={}", r.left_manifest);
            println!("right_manifest={}", r.right_manifest);
            if r.all_pass() {
                ExitCode::SUCCESS
            } else {
                eprintln!("verify failed: {:?}", r.notes);
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("verify error: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_import_gltf(args: Vec<String>) -> ExitCode {
    if args.is_empty() {
        usage();
    }
    let path = PathBuf::from(&args[0]);
    let emit = args.iter().any(|a| a == "--emit-digest");
    match gltf::import_path(&path, &ImportOptions::default()) {
        Ok(r) => {
            if emit {
                print!("{}", r.tables.to_report_json());
            } else {
                println!(
                    "ok scenes={} nodes={} meshes={} primitives={} materials={} textures={}",
                    r.tables.scenes.count,
                    r.tables.nodes.count,
                    r.tables.meshes.count,
                    r.tables.primitives.count,
                    r.tables.materials.count,
                    r.tables.textures.count
                );
            }
            println!("status=ok");
            println!("coverage_complete={}", gltf::coverage_complete(&r.coverage));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error_kind={}", e.kind.as_str());
            eprintln!("message={}", e.message);
            println!("{{\"ok\":false,\"error_kind\":\"{}\"}}", e.kind.as_str());
            ExitCode::from(1)
        }
    }
}

fn cmd_cook_texture(args: Vec<String>) -> ExitCode {
    let mut input: Option<PathBuf> = None;
    let mut fixture: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut profile = CookProfile::WinVulkanBcnV1;
    let mut semantics = TextureSemantics::Color;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                input = Some(PathBuf::from(args.get(i).unwrap_or_else(|| usage())));
            }
            "--fixture" => {
                i += 1;
                fixture = Some(args.get(i).unwrap_or_else(|| usage()).clone());
            }
            "--out" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).unwrap_or_else(|| usage())));
            }
            "--profile" => {
                i += 1;
                let p = args.get(i).unwrap_or_else(|| usage());
                profile = CookProfile::parse(p).unwrap_or_else(|| {
                    eprintln!("unknown profile: {p}");
                    usage();
                });
            }
            "--semantics" => {
                i += 1;
                semantics = match args.get(i).map(|s| s.as_str()) {
                    Some("color") => TextureSemantics::Color,
                    Some("normal") => TextureSemantics::Normal,
                    Some("mask") => TextureSemantics::Mask,
                    other => {
                        eprintln!("unknown semantics: {other:?}");
                        usage();
                    }
                };
            }
            "--emit-source-ppm" => {
                i += 1;
                let p = PathBuf::from(args.get(i).unwrap_or_else(|| usage()));
                let (w, h, rgba) = fixture_checker_rgba16();
                let _ = fs::write(p, encode_ppm_p6(w, h, &rgba));
            }
            other => {
                eprintln!("unknown flag: {other}");
                usage();
            }
        }
        i += 1;
    }

    let out_dir = out.unwrap_or_else(|| usage());
    let (w, h, rgba) = if let Some(name) = fixture {
        match name.as_str() {
            "checker" => fixture_checker_rgba16(),
            "normal" => {
                semantics = TextureSemantics::Normal;
                fixture_normal_rgba16()
            }
            _ => {
                eprintln!("unknown fixture: {name}");
                usage();
            }
        }
    } else if let Some(path) = input {
        let bytes = fs::read(&path).unwrap_or_else(|e| {
            eprintln!("read {}: {e}", path.display());
            std::process::exit(1);
        });
        decode_ppm_p6(&bytes).unwrap_or_else(|e| {
            eprintln!("ppm: {e}");
            std::process::exit(1);
        })
    } else {
        usage();
    };

    match cook_texture(&rgba, w, h, semantics, profile, &out_dir) {
        Ok(r) => {
            println!("codec_version={}", r.codec_version);
            println!("ktx2_digest={}", r.ktx2_digest);
            println!("bcn_digest={}", r.bcn_digest);
            println!("astc_digest={}", r.astc_digest);
            println!("basis_present={}", r.basis_present);
            println!("gpu_format_bcn={}", r.gpu_format_bcn);
            println!("color_max_delta={}", r.color_max_delta);
            println!("normal_length_mad={:.6}", r.normal_length_mad);
            println!("alpha_coverage_delta={:.6}", r.alpha_coverage_delta);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("cook-texture failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        usage();
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "import-gltf" => cmd_import_gltf(args),
        "coverage-list" => {
            for f in gltf::validate::DECLARED_COVERAGE {
                println!("{f}");
            }
            ExitCode::SUCCESS
        }
        "cook-texture" => cmd_cook_texture(args),
        "decode-page" => cmd_decode_page(args),
        "verify" => cmd_verify(args),
        "canon-check" => cmd_canon_check(args),
        _ => {
            eprintln!("unknown command: {cmd}");
            usage();
        }
    }
}
