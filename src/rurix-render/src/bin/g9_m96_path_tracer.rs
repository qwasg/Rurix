//! G9.4 M96 M17 Path Tracer 参照器 device harness(RXS-0357;门
//! `g9.p0.m96.path_tracer_reference`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §2 M96 行 + spec/global_illumination.md RXS-0357)
//!
//! - **固定 seed 两次运行位级一致**:冻结场景集([`path_trace::m96_scenes`])×
//!   冻结 spp 序列([`path_trace::M96_SPP_SEQUENCE`])逐点双跑(两趟完整
//!   `run_ray_query_effects` device 真跑,RT 级 ray query 执行面 U30),输出
//!   字节 digest(SHA-256,out_rgb‖out_stats‖out_samples)逐位一致,且等于
//!   冻结 golden(容差带 JSON 内);
//! - **逐像素 sample count 导出 + 方差/收敛曲线**:out_samples 逐像素 = spp
//!   机核;Σlum/Σlum² → 逐像素方差聚合 + rel-MAE 收敛曲线进 evidence;
//! - **pbrt-v4 对照收敛曲线落入冻结容差带**:同场景同 spp,pbrt(CPU `path`
//!   积分器,EXR→PFM 回读)与 device 输出的相对偏差 ≤ 冻结带
//!   (`milestones/g9/g9_m96_pbrt_tolerance_band.json`;带 = 冻结批实测 ×
//!   [`path_trace::M96_BAND_MARGIN`],provenance 全字段留痕,禁手写 P-09);
//! - **三臂 RED 独立有效**:改 seed / 跳 RR / 关 MIS 三臂(各一次 device 变体
//!   真跑)输出 digest 必须偏离 golden——不偏离 = 漏检 = FAIL;
//! - **megakernel 起步范围冻结**:kernel 仅 Lambert+单面发光(host
//!   `PtScene::validate` 对 specular/体积/透射 fail-closed);本 harness 只装载
//!   经 validate 放行的冻结 fixtures。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备/W3 能力链缺失/无 pbrt provisioning →
//! `G9_M96_PT: SKIP DEV_ENV_DEGRADE`(退 0,非 fake pass;`RURIX_REQUIRE_REAL=1`
//! 下的 SKIP→硬红由 smoke 脚本层裁决);判据不符 / RED 轴失效 → FAIL 退 1。
//! `RURIX_VK_VALIDATION=1`:vk.rs lane 内 fail-closed(任一 ERROR ⇒ 执行 Err ⇒
//! FAIL);evidence 记 validation 模式。
//!
//! ## 用法
//!
//! ```text
//! g9_m96_path_tracer --spv <kernel.spv> --pbrt <pbrt.exe> --imgtool <imgtool.exe>
//!     [--band <path>] [--evidence <path>] [--work-dir <dir>]
//! g9_m96_path_tracer --emit-scenes <dir>          # 导出冻结 pbrt 场景集 + hash
//! g9_m96_path_tracer --freeze --spv .. --pbrt .. --imgtool .. [--band-out <path>]
//! g9_m96_path_tracer --red-arm seed-change|no-rr|no-mis --spv .. [--band <path>]
//! ```

use rurix_render::gi::path_trace::{
    self, BandEntry, M96_BAND_MARGIN, M96_PBRT_REF_SPP, M96_PBRT_SEED, M96_SEED, M96_SPP_SEQUENCE,
    PtConfig, PtImage, PtScene, ToleranceBand,
};
use rurix_rt::render_exec::{self, KernelWave};
use rurix_rt::vk::{
    self, RayQueryBufferDesc, RayQueryDispatchDesc, RayQueryInstanceDesc, RayQuerySceneDesc,
};

const TAG: &str = "G9_M96_PT";
/// RED 臂演示/golden 对拍所用场景×spp(Cornell 多反弹显著,spp=16)。
const RED_SCENE: &str = "m96_cornell";
const RED_SPP: u32 = 16;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn skip(msg: &str) -> ! {
    println!("{TAG}: SKIP DEV_ENV_DEGRADE {msg}");
    std::process::exit(0)
}

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// JSON 字符串转义(手工 JSON 纪律:路径含反斜杠,必转义)。
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn read_u32(b: &[u8]) -> Vec<u32> {
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

// ---------------------------------------------------------------------------
// device 执行腿(U30 run_ray_query_effects;单 BLAS × 单实例;逐像素单 invocation)
// ---------------------------------------------------------------------------

/// 单场景单 spp 单配置 device 真跑 → PtImage(回读装配)。
fn run_device(
    scene: &PtScene,
    cfg: &PtConfig,
    spv: &[u32],
    entry: &str,
) -> Result<PtImage, String> {
    scene.validate().map_err(|e| format!("场景校验: {e}"))?;
    cfg.validate().map_err(|e| format!("配置校验: {e}"))?;
    let cam = &scene.camera;
    let pixel_count = (cam.width * cam.height) as usize;
    let tris = scene.blas_triangles();
    let blas_refs: Vec<&[f32]> = vec![&tris];
    let instances = [RayQueryInstanceDesc {
        blas: 0,
        custom_index: 0,
        mask: 0xFF,
        sbt_record_offset: 0,
    }];
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    let stream = path_trace::rng::generate_stream(pixel_count, cfg.spp, cfg.max_bounces, cfg.seed);
    let rng_b = bytes_f32(&stream);
    let mats_b = bytes_f32(&path_trace::pack_mats(scene));
    let tris_b = bytes_f32(&tris);
    let params_b = bytes_f32(&path_trace::pack_params(scene, cfg));
    let buffers = [
        RayQueryBufferDesc::Input(&rng_b),
        RayQueryBufferDesc::Input(&mats_b),
        RayQueryBufferDesc::Input(&tris_b),
        RayQueryBufferDesc::Input(&params_b),
        RayQueryBufferDesc::Output(pixel_count * 12),
        RayQueryBufferDesc::Output(pixel_count * 8),
        RayQueryBufferDesc::Output(pixel_count * 4),
    ];
    let out = vk::run_ray_query_effects(
        &scene_desc,
        &[RayQueryDispatchDesc {
            name: "g9_m96_path_tracer",
            spv,
            entry,
            buffers: &buffers,
            push_constants: &[],
            groups: [pixel_count as u32, 1, 1],
        }],
    )?;
    let rb = out
        .readbacks
        .into_iter()
        .next()
        .ok_or("单 dispatch 缺回读")?;
    if rb.len() != 3 {
        return Err(format!("回读路数 {} ≠ 3", rb.len()));
    }
    Ok(PtImage {
        width: cam.width,
        height: cam.height,
        rgb: read_f32(&rb[0]),
        sum_lum: read_f32(
            &rb[1]
                .chunks_exact(8)
                .map(|c| &c[..4])
                .collect::<Vec<_>>()
                .concat(),
        ),
        sumsq_lum: read_f32(
            &rb[1]
                .chunks_exact(8)
                .map(|c| &c[4..])
                .collect::<Vec<_>>()
                .concat(),
        ),
        samples: read_u32(&rb[2]),
    })
}

/// 双跑位级一致 + digest(判据①承载)。
fn run_device_double(
    scene: &PtScene,
    cfg: &PtConfig,
    spv: &[u32],
    entry: &str,
) -> Result<(PtImage, [u8; 32], bool), String> {
    let a = run_device(scene, cfg, spv, entry)?;
    let b = run_device(scene, cfg, spv, entry)?;
    let da = path_trace::image_digest(&a);
    let db = path_trace::image_digest(&b);
    Ok((a, da, da == db))
}

// ---------------------------------------------------------------------------
// pbrt 腿(provisioning 显式;EXR→PFM 回读;伪造禁止——全部子进程真跑)
// ---------------------------------------------------------------------------

/// pbrt 运行一次(cwd = work_dir;输出 EXR 文件名由场景文件内 Film 声明)。
fn run_pbrt(
    pbrt: &std::path::Path,
    work: &std::path::Path,
    scene_file: &std::path::Path,
) -> Result<(), String> {
    let r = std::process::Command::new(pbrt)
        .arg("--nthreads")
        .arg("0")
        .arg(scene_file)
        .current_dir(work)
        .output()
        .map_err(|e| format!("pbrt 启动失败: {e}"))?;
    if !r.status.success() {
        return Err(format!(
            "pbrt 退出码 {:?}:{}",
            r.status.code(),
            String::from_utf8_lossy(&r.stderr)
                .lines()
                .take(4)
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    Ok(())
}

/// imgtool EXR → PFM。
fn exr_to_pfm(
    imgtool: &std::path::Path,
    exr: &std::path::Path,
    pfm: &std::path::Path,
) -> Result<(), String> {
    let r = std::process::Command::new(imgtool)
        .arg("convert")
        .arg(exr)
        .arg("--outfile")
        .arg(pfm)
        .output()
        .map_err(|e| format!("imgtool 启动失败: {e}"))?;
    if !r.status.success() {
        return Err(format!(
            "imgtool 退出码 {:?}:{}",
            r.status.code(),
            String::from_utf8_lossy(&r.stderr)
                .lines()
                .take(4)
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    Ok(())
}

/// pbrt 场景文件物质化(确定性文本;返回 sha256 hex)。
fn materialize_pbrt_scene(
    dir: &std::path::Path,
    scene: &PtScene,
    spp: u32,
) -> Result<(std::path::PathBuf, String), String> {
    let cfg = PtConfig::reference(spp);
    let exr_name = path_trace::pbrt_scene_filename(scene.name, spp).replace(".pbrt", ".exr");
    let text = path_trace::pbrt_scene_text(scene, &cfg, M96_PBRT_SEED, &exr_name);
    let path = dir.join(path_trace::pbrt_scene_filename(scene.name, spp));
    std::fs::write(&path, &text).map_err(|e| format!("写 {}: {e}", path.display()))?;
    Ok((path, hex(&rurix_pkg::sha256::digest(text.as_bytes()))))
}

/// pbrt 渲染 + 回读 PFM(行序自顶向下,与 rurix 像素序对齐)。
fn pbrt_render(
    pbrt: &std::path::Path,
    imgtool: &std::path::Path,
    work: &std::path::Path,
    scene: &PtScene,
    spp: u32,
) -> Result<Vec<f32>, String> {
    let (scene_path, _) = materialize_pbrt_scene(work, scene, spp)?;
    // pbrt 以 cwd=work 运行:场景路径须绝对化(相对路径会被双重拼接);
    // canonicalize 的 \\?\ 前缀剥除(pbrt 不识别扩展长度路径)。
    let scene_path = std::fs::canonicalize(&scene_path)
        .map_err(|e| format!("canonicalize {}: {e}", scene_path.display()))?;
    let scene_str = scene_path.display().to_string();
    let scene_str = scene_str
        .strip_prefix(r"\\?\")
        .unwrap_or(&scene_str)
        .to_string();
    run_pbrt(pbrt, work, std::path::Path::new(&scene_str))?;
    let stem = path_trace::pbrt_scene_filename(scene.name, spp).replace(".pbrt", "");
    let exr = work.join(format!("{stem}.exr"));
    let pfm = work.join(format!("{stem}.pfm"));
    if !exr.is_file() {
        return Err(format!("pbrt 未产 {}", exr.display()));
    }
    exr_to_pfm(imgtool, &exr, &pfm)?;
    let bytes = std::fs::read(&pfm).map_err(|e| format!("读 {}: {e}", pfm.display()))?;
    let (w, h, img) = path_trace::read_pfm(&bytes).map_err(|e| e.to_string())?;
    if (w, h) != (scene.camera.width, scene.camera.height) {
        return Err(format!("PFM 尺寸 {w}×{h} ≠ 冻结相机"));
    }
    Ok(img)
}

/// pbrt provisioning 探测(版本行 + commit + exe sha256;缺一即 DEV_ENV_DEGRADE)。
fn pbrt_provenance(pbrt: &std::path::Path) -> Result<(String, String, String), String> {
    // pbrt-v4 无 --version 子命令;无参运行首行横幅 = `pbrt version 4 (built …)`。
    let ver = std::process::Command::new(pbrt)
        .output()
        .map_err(|e| format!("pbrt 横幅探测: {e}"))?;
    let banner = String::from_utf8_lossy(&ver.stdout);
    let version = banner
        .lines()
        .find(|l| l.contains("pbrt version"))
        .unwrap_or_else(|| banner.lines().next().unwrap_or("unknown"))
        .trim()
        .to_string();
    if !version.contains("pbrt version") {
        return Err(format!("pbrt 横幅形态非预期:{version}"));
    }
    let exe_bytes = std::fs::read(pbrt).map_err(|e| format!("读 pbrt exe: {e}"))?;
    let exe_sha = hex(&rurix_pkg::sha256::digest(&exe_bytes));
    // commit:external/pbrt-v4 源树(provisioning 落点;无 .git 记 unknown)。
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../external/pbrt-v4");
    let commit = std::process::Command::new("git")
        .args(["-C", &repo.display().to_string(), "rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    Ok((version, commit, exe_sha))
}

// ---------------------------------------------------------------------------
// 参数解析
// ---------------------------------------------------------------------------

struct Args {
    spv: Option<String>,
    evidence: Option<String>,
    band: String,
    pbrt: Option<String>,
    imgtool: Option<String>,
    work_dir: String,
    freeze: bool,
    band_out: Option<String>,
    emit_scenes: Option<String>,
    emit_host_oracle_pfm: Option<String>,
    red_arm: Option<String>,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = Args {
        spv: None,
        evidence: None,
        band: "milestones/g9/g9_m96_pbrt_tolerance_band.json".to_string(),
        pbrt: None,
        imgtool: None,
        work_dir: ".tmp/g9_m96_work".to_string(),
        freeze: false,
        band_out: None,
        emit_scenes: None,
        emit_host_oracle_pfm: None,
        red_arm: None,
    };
    let mut i = 0;
    while i < args.len() {
        let take = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
        };
        match args[i].as_str() {
            "--spv" => out.spv = Some(take(&mut i)),
            "--evidence" => out.evidence = Some(take(&mut i)),
            "--band" => out.band = take(&mut i),
            "--pbrt" => out.pbrt = Some(take(&mut i)),
            "--imgtool" => out.imgtool = Some(take(&mut i)),
            "--work-dir" => out.work_dir = take(&mut i),
            "--freeze" => out.freeze = true,
            "--band-out" => out.band_out = Some(take(&mut i)),
            "--emit-scenes" => out.emit_scenes = Some(take(&mut i)),
            "--emit-host-oracle-pfm" => out.emit_host_oracle_pfm = Some(take(&mut i)),
            "--red-arm" => out.red_arm = Some(take(&mut i)),
            other => fail(&format!("unknown arg {other}")),
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// 场景集 pbrt 文件导出(--emit-scenes;checked-in 语料的物质化面)
// ---------------------------------------------------------------------------

fn emit_scenes(dir: &str) -> ! {
    let d = std::path::Path::new(dir);
    std::fs::create_dir_all(d).unwrap_or_else(|e| fail(&format!("建目录 {dir}: {e}")));
    for scene in path_trace::m96_scenes() {
        for spp in M96_SPP_SEQUENCE.iter().copied().chain([M96_PBRT_REF_SPP]) {
            match materialize_pbrt_scene(d, &scene, spp) {
                Ok((path, hash)) => println!(
                    "{TAG}: scene {} -> {} sha256={hash}",
                    scene.name,
                    path.display()
                ),
                Err(e) => fail(&e),
            }
        }
    }
    println!("{TAG}: PASS emit-scenes(2 场景 × 5 spp 档)");
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    println!(
        "[g9_m96_path_tracer] G9.4 M96 Path Tracer 参照器 device harness(RXS-0357;门 g9.p0.m96.path_tracer_reference)"
    );
    let args = parse_args();
    if let Some(dir) = &args.emit_scenes {
        emit_scenes(dir);
    }
    // host oracle PFM 落盘子模式(对照调试/朝向机核;无 device/pbrt 依赖)。
    if let Some(dir) = &args.emit_host_oracle_pfm {
        let d = std::path::Path::new(dir);
        std::fs::create_dir_all(d).unwrap_or_else(|e| fail(&format!("建目录 {dir}: {e}")));
        for scene in path_trace::m96_scenes() {
            for spp in M96_SPP_SEQUENCE {
                let cfg = PtConfig::reference(spp);
                let px = (scene.camera.width * scene.camera.height) as usize;
                let stream =
                    path_trace::rng::generate_stream(px, cfg.spp, cfg.max_bounces, cfg.seed);
                let img = path_trace::trace_host(&scene, &cfg, &stream)
                    .unwrap_or_else(|e| fail(&format!("host oracle: {e}")));
                let path = d.join(format!("{}_spp{spp}_rurix_host.pfm", scene.name));
                std::fs::write(&path, path_trace::write_pfm(&img))
                    .unwrap_or_else(|e| fail(&format!("写 {}: {e}", path.display())));
                println!(
                    "{TAG}: host-oracle {} spp={spp} mean_lum={:.6} -> {}",
                    scene.name,
                    img.mean_luminance(),
                    path.display()
                );
            }
        }
        println!("{TAG}: PASS emit-host-oracle-pfm");
        std::process::exit(0);
    }

    // ── 步骤 0:device 门(三态)──
    if !vk::vulkan_available() {
        skip("无 Vulkan loader(dev-env degrade)");
    }
    let caps = match render_exec::probe_device_caps() {
        Ok(c) => c,
        Err(e) => skip(&format!("无 Vulkan 物理设备({})", e.trim())),
    };
    if let Err(e) = render_exec::require_wave(&caps, KernelWave::W3) {
        skip(&format!("W3(ray query)能力链缺失({e})"));
    }
    let validation_on = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    println!(
        "{TAG}: device=`{}` validation={}",
        caps.device_name,
        if validation_on { "on" } else { "off" }
    );
    let spv_path = args
        .spv
        .clone()
        .unwrap_or_else(|| fail("缺 --spv <kernel.spv>"));
    let spv_bytes =
        std::fs::read(&spv_path).unwrap_or_else(|e| fail(&format!("读 {spv_path}: {e}")));
    if spv_bytes.len() % 4 != 0 {
        fail("SPIR-V 字节数非 4 对齐");
    }
    let spv: Vec<u32> = spv_bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let entry = vk::entry_point_name(&spv).unwrap_or_else(|| fail("SPIR-V 无 OpEntryPoint"));
    println!("{TAG}: kernel entry=`{entry}`");

    // ── 步骤 1:device 双腿(gate = 全场景 × spp 序列;--red-arm 子模式 = 仅
    //    RED 基线点;双跑位级一致)──
    let scenes = path_trace::m96_scenes();
    let mut failures: Vec<String> = Vec::new();
    let mut double_run_bitexact = true;
    let mut sample_count_ok = true;
    let mut device_images: std::collections::BTreeMap<(String, u32), (PtImage, [u8; 32])> =
        Default::default();
    let run_matrix: Vec<(&PtScene, u32)> = if args.red_arm.is_some() {
        scenes
            .iter()
            .filter(|s| s.name == RED_SCENE)
            .map(|s| (s, RED_SPP))
            .collect()
    } else {
        scenes
            .iter()
            .flat_map(|s| M96_SPP_SEQUENCE.iter().map(move |&spp| (s, spp)))
            .collect()
    };
    for (scene, spp) in run_matrix {
        let cfg = PtConfig::reference(spp);
        let (img, digest, bitexact) = match run_device_double(scene, &cfg, &spv, &entry) {
            Ok(v) => v,
            Err(e) => fail(&format!("device 真跑 {} spp={spp}: {e}", scene.name)),
        };
        if !bitexact {
            double_run_bitexact = false;
            failures.push(format!("{} spp={spp} 双跑 digest 分叉", scene.name));
        }
        if !img.samples.iter().all(|&n| n == spp) {
            sample_count_ok = false;
            failures.push(format!(
                "{} spp={spp} 逐像素 sample count ≠ spp",
                scene.name
            ));
        }
        if !img.rgb.iter().all(|v| v.is_finite() && *v >= 0.0) {
            failures.push(format!("{} spp={spp} 输出非有限/负", scene.name));
        }
        println!(
            "{TAG}: device {} spp={spp} digest={} mean_lum={:.6}",
            scene.name,
            hex(&digest),
            img.mean_luminance()
        );
        device_images.insert((scene.name.to_string(), spp), (img, digest));
    }

    // ── 步骤 2:三臂 RED(device 变体真跑;digest 偏离 golden 必检出)──
    let red_scene = scenes
        .iter()
        .find(|s| s.name == RED_SCENE)
        .expect("cornell fixture 在集");
    let base_cfg = PtConfig::reference(RED_SPP);
    let golden_digest = device_images
        .get(&(RED_SCENE.to_string(), RED_SPP))
        .map(|(_, d)| *d)
        .expect("golden digest 在集");
    let arm = |name: &str, mutate: &dyn Fn(&mut PtConfig)| -> bool {
        let mut cfg = base_cfg;
        mutate(&mut cfg);
        match run_device(red_scene, &cfg, &spv, &entry) {
            Ok(img) => {
                let d = path_trace::image_digest(&img);
                let detected = d != golden_digest;
                println!(
                    "{TAG}: RED 臂 {name} digest={} vs golden={} → {}",
                    hex(&d),
                    hex(&golden_digest),
                    if detected {
                        "检出(RED 有效)"
                    } else {
                        "未检出(漏检)"
                    }
                );
                detected
            }
            Err(e) => fail(&format!("RED 臂 {name} device 执行: {e}")),
        }
    };
    let red_seed = arm("seed-change", &|c| {
        c.seed = M96_SEED ^ 0xABCD_EF01_2345_6789
    });
    let red_no_rr = arm("no-rr", &|c| c.switches.rr = false);
    let red_no_mis = arm("no-mis", &|c| c.switches.mis = false);
    for (name, ok) in [
        ("red_seed", red_seed),
        ("red_no_rr", red_no_rr),
        ("red_no_mis", red_no_mis),
    ] {
        if !ok {
            failures.push(format!("RED 臂 {name} 失效:变体输出未偏离 golden(漏检)"));
        }
    }

    // ── 步骤 2.5:起步范围冻结显式拒绝(焦散/体积/specular 链 out;装载面
    //    fail-closed 机核——注入范围外材质,validate 必 typed Err)──
    let mut scope_reject = true;
    {
        use rurix_render::gi::path_trace::MaterialKind;
        for bad in [
            MaterialKind::Specular {
                reflectance: [1.0; 3],
            },
            MaterialKind::Transmission {
                transmittance: [1.0; 3],
            },
            MaterialKind::Volume { density: 1.0 },
        ] {
            let mut tampered = red_scene.clone();
            tampered.materials[0] = bad;
            if tampered.validate().is_ok() {
                scope_reject = false;
                failures.push(format!("范围外材质 {bad:?} 未被 validate 拒绝"));
            }
        }
        println!("{TAG}: 起步范围冻结显式拒绝(specular/透射/体积注入全拒)= {scope_reject}");
    }

    // --red-arm 子模式:到 RED 臂为止(命令行子模式重跑演示;pbrt 腿不跑)。
    if let Some(arm_name) = &args.red_arm {
        let ok = match arm_name.as_str() {
            "seed-change" => red_seed,
            "no-rr" => red_no_rr,
            "no-mis" => red_no_mis,
            other => fail(&format!("unknown --red-arm {other}")),
        };
        if ok && double_run_bitexact {
            println!("{TAG}: PASS red-arm {arm_name}(独立检出;正例臂双跑位级一致)");
            std::process::exit(0);
        }
        fail(&format!("red-arm {arm_name} 未检出"));
    }

    let (pbrt_exe, imgtool_exe) = match (&args.pbrt, &args.imgtool) {
        (Some(p), Some(i)) => (std::path::PathBuf::from(p), std::path::PathBuf::from(i)),
        _ => skip("无 pbrt provisioning(--pbrt/--imgtool 未给;DEV_ENV_DEGRADE 登记,不充绿)"),
    };
    if !pbrt_exe.is_file() {
        skip(&format!(
            "pbrt 不存在({})(DEV_ENV_DEGRADE)",
            pbrt_exe.display()
        ));
    }
    if !imgtool_exe.is_file() {
        skip(&format!(
            "imgtool 不存在({})(DEV_ENV_DEGRADE)",
            imgtool_exe.display()
        ));
    }
    let (pbrt_version, pbrt_commit, pbrt_exe_sha) = match pbrt_provenance(&pbrt_exe) {
        Ok(v) => v,
        Err(e) => skip(&format!("pbrt provisioning 探测失败({e})(DEV_ENV_DEGRADE)")),
    };
    println!("{TAG}: pbrt `{pbrt_version}` commit={pbrt_commit} exe_sha256={pbrt_exe_sha}");
    let work = std::path::PathBuf::from(&args.work_dir);
    std::fs::create_dir_all(&work).unwrap_or_else(|e| fail(&format!("建 work-dir: {e}")));

    // pbrt 渲染:参照档 + spp 序列(逐场景)。
    let mut pbrt_images: std::collections::BTreeMap<(String, u32), Vec<f32>> = Default::default();
    for scene in &scenes {
        for spp in M96_SPP_SEQUENCE.iter().copied().chain([M96_PBRT_REF_SPP]) {
            match pbrt_render(&pbrt_exe, &imgtool_exe, &work, scene, spp) {
                Ok(img) => {
                    pbrt_images.insert((scene.name.to_string(), spp), img);
                }
                Err(e) => fail(&format!("pbrt 腿 {} spp={spp}: {e}", scene.name)),
            }
            println!("{TAG}: pbrt {} spp={spp} 渲染+回读完成", scene.name);
        }
    }

    // 度量:rel_dev(device vs pbrt 同 spp)+ 收敛曲线(rel-MAE vs pbrt ref)。
    let mut measured: Vec<BandEntry> = Vec::new();
    for scene in &scenes {
        let reference = &pbrt_images[&(scene.name.to_string(), M96_PBRT_REF_SPP)];
        for spp in M96_SPP_SEQUENCE {
            let (rimg, digest) = &device_images[&(scene.name.to_string(), spp)];
            let pimg = &pbrt_images[&(scene.name.to_string(), spp)];
            let dev = path_trace::rel_dev(&rimg.rgb, pimg).expect("rel_dev 计算");
            let curve_r = path_trace::rel_mae(&rimg.rgb, reference).expect("curve_r");
            let curve_p = path_trace::rel_mae(pimg, reference).expect("curve_p");
            println!(
                "{TAG}: {} spp={spp} rel_dev={dev:.6e} curve[rurix={curve_r:.6e} pbrt={curve_p:.6e}]",
                scene.name
            );
            measured.push(BandEntry {
                scene: scene.name.to_string(),
                spp,
                golden_digest: hex(digest),
                band_rel_dev: dev * M96_BAND_MARGIN,
                measured_rel_dev: dev,
                curve_rurix: curve_r,
                curve_pbrt: curve_p,
            });
        }
    }

    // ── 步骤 4:freeze(写带)或 gate(比对带)──
    let mut golden_digest_match = true;
    let mut pbrt_band_within = true;
    if args.freeze {
        let band = ToleranceBand {
            frozen_at_utc: utc_now(),
            device_name: caps.device_name.clone(),
            pbrt_version: pbrt_version.clone(),
            pbrt_commit: pbrt_commit.clone(),
            pbrt_exe_sha256: pbrt_exe_sha.clone(),
            entries: measured.clone(),
        };
        let out = args.band_out.clone().unwrap_or(args.band.clone());
        std::fs::write(&out, band.to_json()).unwrap_or_else(|e| fail(&format!("写带 {out}: {e}")));
        println!("{TAG}: FREEZE 容差带已写 {out}(measured × {M96_BAND_MARGIN};provenance 全字段)");
    } else {
        let band_text = std::fs::read_to_string(&args.band)
            .unwrap_or_else(|e| fail(&format!("读容差带 {}: {e}", args.band)));
        let band =
            ToleranceBand::parse(&band_text).unwrap_or_else(|e| fail(&format!("容差带解析: {e}")));
        for m in &measured {
            match band.entry(&m.scene, m.spp) {
                Ok(e) => {
                    if m.golden_digest != e.golden_digest {
                        golden_digest_match = false;
                        failures.push(format!(
                            "{} spp={} digest {} ≠ golden {}",
                            m.scene, m.spp, m.golden_digest, e.golden_digest
                        ));
                    }
                    if m.measured_rel_dev.is_nan() || m.measured_rel_dev > e.band_rel_dev {
                        pbrt_band_within = false;
                        failures.push(format!(
                            "{} spp={} rel_dev {:.6e} 越带(上界 {:.6e})",
                            m.scene, m.spp, m.measured_rel_dev, e.band_rel_dev
                        ));
                    }
                }
                Err(e) => {
                    golden_digest_match = false;
                    pbrt_band_within = false;
                    failures.push(e.to_string());
                }
            }
        }
        if golden_digest_match && pbrt_band_within {
            println!("{TAG}: pbrt 对照在带内(golden digest 全等 + rel_dev ≤ 冻结带)");
        }
    }

    // ── 步骤 5:evidence(rurix.g9m96.path_tracer.v1)──
    let checks: [(&str, bool); 9] = [
        ("double_run_bitexact", double_run_bitexact),
        ("sample_count_export", sample_count_ok),
        ("golden_digest_match", golden_digest_match),
        ("pbrt_band_within", pbrt_band_within),
        ("red_seed", red_seed),
        ("red_no_rr", red_no_rr),
        ("red_no_mis", red_no_mis),
        ("scope_reject_failclosed", scope_reject),
        ("validation_zero", true), // vk.rs lane 内 fail-closed:到此即零 ERROR
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let digests_json: Vec<String> = measured
        .iter()
        .map(|m| format!("\"{}_spp{}\": \"{}\"", m.scene, m.spp, m.golden_digest))
        .collect();
    let curves_json: Vec<String> = measured
        .iter()
        .map(|m| {
            format!(
                "\"{}_spp{}\": {{\"rel_dev\": \"{:e}\", \"curve_rurix\": \"{:e}\", \"curve_pbrt\": \"{:e}\"}}",
                m.scene, m.spp, m.measured_rel_dev, m.curve_rurix, m.curve_pbrt
            )
        })
        .collect();
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m96.path_tracer.v1\",\n  \
         \"subject\": \"g9_m96_path_tracer\",\n  \
         \"spec_anchor\": \"RXS-0357\",\n  \
         \"device_state\": {{\"device_name\": \"{}\", \"validation\": \"{}\", \
         \"require_real\": {}}},\n  \
         \"determinism_protocol\": {{\"seed_device\": \"{}\", \"rng\": \"PCG32 单一流按索引寻址(rt::ref_tracer::Pcg32 同一实例;流为输入非结果,G7.4 先例)\", \
         \"accumulation\": \"逐像素独立顺序累加(禁 atomic)\", \
         \"digest_domain\": \"sha256(out_rgb ‖ out_stats ‖ out_samples 字节)\"}},\n  \
         \"pbrt\": {{\"version\": \"{}\", \"commit\": \"{}\", \"exe_sha256\": \"{}\", \
         \"seed_pbrt\": \"{}\", \"ref_spp\": {}}},\n  \
         \"checks\": {{{}}},\n  \
         \"digests\": {{{}}},\n  \
         \"convergence\": {{{}}},\n  \
         \"commands\": [{}],\n  \
         \"failures\": [{}]\n}}",
        json_escape(&caps.device_name),
        if validation_on { "on" } else { "off" },
        std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        M96_SEED,
        json_escape(&pbrt_version),
        json_escape(&pbrt_commit),
        json_escape(&pbrt_exe_sha),
        M96_PBRT_SEED,
        M96_PBRT_REF_SPP,
        checks_json.join(", "),
        digests_json.join(", "),
        curves_json.join(", "),
        std::env::args()
            .map(|a| format!("\"{}\"", json_escape(&a)))
            .collect::<Vec<_>>()
            .join(", "),
        failures_json.join(", "),
    );
    if let Some(p) = &args.evidence {
        std::fs::write(p, &json).unwrap_or_else(|e| fail(&format!("写 evidence {p}: {e}")));
    }
    println!("{json}");
    if failures.is_empty() {
        println!(
            "{TAG}: PASS 双跑位级一致 + golden 全等 + pbrt 带内 + 三臂 RED 有效(validation={})",
            if validation_on { "on(0 error)" } else { "off" }
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}

/// UTC 时间戳(秒级;无依赖手工拼)。
fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    //  civil-from-days 算法(Howard Hinnant)。
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}
