//! rxcook — G8.3 资产 cook CLI(设计案 §1.2)。
//! 子命令:`import-gltf`(M81) / `cook-texture`(M83) / `decode-page`(M04) /
//! `verify --double-build`(M79) / `coverage-list`。

use rurix_asset::gltf::{self, validate::ImportOptions};
use rurix_asset::texture::{
    CookProfile, TextureSemantics, cook_texture, decode_ppm_p6, encode_ppm_p6,
    fixture_checker_rgba16, fixture_normal_rgba16,
};
use rurix_asset::canon;
use rurix_asset::ddc::{self, Ddc, GetMiss, PutError};
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
        "usage:\n  rxcook import-gltf <path> [--emit-digest]\n  rxcook coverage-list\n  rxcook cook-texture ...\n  rxcook decode-page ...\n  rxcook verify --double-build ...\n  rxcook canon-check ...\n  rxcook ddc-selftest [--scratch <dir>]\n  rxcook ddc-manifest-phase --digest <hex> --flip-digest <hex> [--scratch <dir>]"
    );
    std::process::exit(2);
}

fn cmd_ddc_selftest(args: Vec<String>) -> ExitCode {
    let mut scratch = env::temp_dir().join(format!("rxcook_ddc_{}", std::process::id()));
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scratch" => {
                i += 1;
                scratch = PathBuf::from(args.get(i).unwrap_or_else(|| usage()));
            }
            other => {
                eprintln!("unknown ddc-selftest flag: {other}");
                usage();
            }
        }
        i += 1;
    }
    let _ = fs::remove_dir_all(&scratch);
    let mut checks: Vec<(&str, bool)> = Vec::new();

    let base = match ddc::demo_segments("base") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    let k0 = ddc::compute_key(&base).unwrap();
    let k0b = ddc::compute_key(&base).unwrap();
    checks.push(("same_preimage_same_key", k0 == k0b));
    checks.push((
        "preimage_covers_source_digest",
        ddc::compute_key(&ddc::mutate_segment(&base, 0).unwrap()).unwrap() != k0,
    ));
    checks.push((
        "preimage_covers_dependency_keys",
        ddc::compute_key(&ddc::mutate_segment(&base, 1).unwrap()).unwrap() != k0,
    ));
    checks.push((
        "preimage_covers_tool_version",
        ddc::compute_key(&ddc::mutate_segment(&base, 4).unwrap()).unwrap() != k0,
    ));
    checks.push((
        "preimage_covers_cook_profile",
        ddc::compute_key(&ddc::mutate_segment(&base, 3).unwrap()).unwrap() != k0,
    ));

    let names = [
        "mutation_source_flips_key",
        "mutation_dependency_flips_key",
        "mutation_recipe_flips_key",
        "mutation_profile_flips_key",
        "mutation_toolchain_flips_key",
        "mutation_schema_set_flips_key",
        "mutation_abi_set_flips_key",
        "mutation_artifact_kind_flips_key",
        "mutation_output_id_flips_key",
    ];
    for (i, name) in names.iter().enumerate() {
        let m = ddc::mutate_segment(&base, i).unwrap();
        checks.push((name, ddc::compute_key(&m).unwrap() != k0));
    }

    let mut ddc_store = Ddc::open(&scratch).unwrap();
    let payload = b"ddc-payload-v1";
    let meta = ddc::make_meta_envelope(payload).unwrap();
    ddc_store.put(&k0, payload, &meta).unwrap();
    let got = ddc_store.get(&k0).unwrap();
    checks.push(("put_get_byte_equal", got == payload));

    // bitflip
    let obj = scratch
        .join("objects")
        .join(&k0.hex()[..2])
        .join(k0.hex());
    let mut bytes = fs::read(&obj).unwrap();
    bytes[0] ^= 0xff;
    fs::write(&obj, &bytes).unwrap();
    checks.push((
        "bitflip_rejected_as_corruption",
        matches!(ddc_store.get(&k0), Err(GetMiss::Corruption { .. })),
    ));
    // restore + truncate
    fs::write(&obj, payload).unwrap();
    // rewrite meta for restore
    let meta2 = ddc::make_meta_envelope(payload).unwrap();
    let meta_p = scratch
        .join("meta")
        .join(&k0.hex()[..2])
        .join(format!("{}.rxap", k0.hex()));
    fs::write(&meta_p, &meta2).unwrap();
    fs::write(&obj, &payload[..4]).unwrap();
    checks.push((
        "truncation_rejected",
        matches!(ddc_store.get(&k0), Err(GetMiss::Corruption { .. })),
    ));

    // rebuild clean
    let _ = fs::remove_dir_all(&scratch);
    let mut ddc_store = Ddc::open(&scratch).unwrap();
    ddc_store.put(&k0, payload, &meta).unwrap();
    // collision
    let coll = matches!(
        ddc_store.put(&k0, b"other-payload!!", &meta),
        Err(PutError::KeyCollision)
    );
    checks.push(("concurrent_same_key_put_safe", {
        // same payload put ok
        ddc_store.put(&k0, payload, &meta).is_ok() && coll
    }));

    ddc_store.evict(&k0).unwrap();
    let miss = matches!(ddc_store.get(&k0), Err(GetMiss::Absent));
    ddc_store.put(&k0, payload, &meta).unwrap();
    let again = ddc_store.get(&k0).unwrap();
    checks.push((
        "evict_then_rebuild_key_stable",
        miss && again == payload && ddc::compute_key(&base).unwrap() == k0,
    ));

    println!("{{");
    for (i, (k, v)) in checks.iter().enumerate() {
        let comma = if i + 1 == checks.len() { "" } else { "," };
        println!("  \"{k}\": {v}{comma}");
    }
    println!("}}");
    let ok = checks.iter().all(|(_, v)| *v);
    println!("ok={ok}");
    println!("check_count={}", checks.len());
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
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
        "ddc-selftest" => cmd_ddc_selftest(args),
        "ddc-manifest-phase" => cmd_ddc_manifest_phase(args),
        _ => {
            eprintln!("unknown command: {cmd}");
            usage();
        }
    }
}

fn cmd_ddc_manifest_phase(args: Vec<String>) -> ExitCode {
    use rurix_asset::canon::Value;
    let mut digest = None;
    let mut flip = None;
    let mut scratch = env::temp_dir().join(format!("ddc_m85_{}", std::process::id()));
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--digest" => {
                i += 1;
                digest = Some(args.get(i).unwrap_or_else(|| usage()).clone());
            }
            "--flip-digest" => {
                i += 1;
                flip = Some(args.get(i).unwrap_or_else(|| usage()).clone());
            }
            "--scratch" => {
                i += 1;
                scratch = PathBuf::from(args.get(i).unwrap_or_else(|| usage()));
            }
            other => {
                eprintln!("unknown ddc-manifest-phase flag: {other}");
                usage();
            }
        }
        i += 1;
    }
    let digest = digest.unwrap_or_else(|| usage());
    let flip = flip.unwrap_or_else(|| usage());
    let _ = fs::remove_dir_all(&scratch);
    let mut store = match Ddc::open(&scratch) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    let segs = |d: &str| {
        ddc::PreimageSegments {
            source_set: Value::map_of([(1, Value::text_ascii("shader.manifest").unwrap())]).unwrap(),
            dependency_keys: Value::Array(vec![Value::text_ascii(d).unwrap()]),
            import_recipe: Value::map_of([(1, Value::text_ascii(d).unwrap())]).unwrap(),
            cook_profile: Value::map_of([(1, Value::text_ascii("g8.3").unwrap())]).unwrap(),
            tool_chain: Value::map_of([(1, Value::text_ascii("rurixc").unwrap())]).unwrap(),
            schema_set: Value::Array(vec![Value::text_ascii("shader-manifest.v1").unwrap()]),
            abi_set: Value::Array(vec![Value::text_ascii("abi.v1").unwrap()]),
            artifact_kind: Value::text_ascii("shader.manifest").unwrap(),
            output_id: Value::text_ascii("merged").unwrap(),
        }
    };
    let s0 = segs(&digest);
    let k0 = ddc::compute_key(&s0).unwrap();
    let payload = digest.as_bytes();
    let meta = ddc::make_meta_envelope(payload).unwrap();
    store.put(&k0, payload, &meta).unwrap();
    let got = store.get(&k0).unwrap();
    let put_get = got == payload;
    let s1 = segs(&flip);
    let k1 = ddc::compute_key(&s1).unwrap();
    let key_flip = k1 != k0;
    let old_hit = store.get(&k0).unwrap() == payload;
    let new_miss = matches!(store.get(&k1), Err(GetMiss::Absent));
    println!("preimage_covers_digest=true");
    println!("put_get={put_get}");
    println!("key_flip={key_flip}");
    println!("old_hit={}", old_hit && new_miss);
    if put_get && key_flip && old_hit && new_miss {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
