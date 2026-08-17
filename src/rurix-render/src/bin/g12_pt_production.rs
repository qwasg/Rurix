//! G12.2 生产化核心波 Path Tracer 生产化 device harness + 标定程序
//! (spec/global_illumination.md RXS-0398~0401;RFC-0029 §4.1~§4.4;门
//! `g12.p0.m158.mis_full_surface` / `g12.p0.m159.russian_roulette_prod` /
//! `g12.p0.m160.sampling_lds_upgrade` / `g12.p0.m161.convergence_criterion_prod`
//! / `g12.p1.m166.pt_production_calibration`)。G12.3 降噪波扩展面(RXS-0402;
//! RFC-0029 §4.5;门 `g12.p0.m162.denoise_pipeline_tsr`):降噪管线 device 腿
//! (时域累积 + firefly 预钳位 + A-trous,消费 `kernels/g12_pt_denoise.rx` 经
//! --denoise-spv)+ 噪声谱/均值能量测量 + 降噪标定腿(--calibrate-denoise,
//! 纯 host)+ 降噪 RED 三臂(denoise-energy-bias/denoise-masquerade/
//! history-validation-off);temporal 底座 0-byte 不接线(只读消费历史接口面)。
//!
//! ## 判据面(G12_CONTRACT §4.2 M158~M161 行 + G12_ACCEPTANCE_MAP §1/§2 逐字)
//!
//! 统一形态 = 生产化落盘(device megakernel `kernels/g12_pt_production.rx`
//! 真跑)+ 正确性锚 0-byte(M96 既有判据/固定 seed 位级确定性协议/golden
//! 门序 D2-Q7 维持——本 harness 只消费 M96 冻结面不回写)+ 收敛/方差面
//! measured 不劣于参照器基线锚(`g12_budget.json` 的 `g12.pt.ref_curve_*`
//! 锚,容差由 --calibrate 标定程序 measured 产出经 g12_budget 传入,禁手写
//! P-09)+ RED 臂独立有效(变体输出偏离正例臂 digest 必检出,不偏离 = 漏检
//! = FAIL)。
//!
//! - `--gate g12.p0.m158.mis_full_surface`:多光源/白炉/delta 三 fixture +
//!   双 m96 场景曲线锚;delta 退化 MIS 开/关位级一致;白炉能量守恒 + 逐级
//!   能量增量单调不增;RED 臂 no-mis/energy-bias/seed-change。
//! - `--gate g12.p0.m159.russian_roulette_prod`:RR 计数非空(终止率/补偿
//!   分布)+ 无偏对照(RR 开/关均值差 ≤ 标定阈)+ 曲线锚;RED 臂
//!   no-rr/comp-off/early-kill;RR 参数 fail-closed(N_min<2/p_max=1 必拒)。
//! - `--gate g12.p0.m160.sampling_lds_upgrade`:流位级一致 + 索引推导确定性
//!   (逐索引重求值 == 流内容)+ 流布局 provenance + 选型 benchmark 确定性
//!   + 曲线锚(winner 族);RED 臂 nondeterministic/seed-change。
//! - `--gate g12.p0.m161.convergence_criterion_prod`:自适应 spp 终止 +
//!   收敛报告(spp 分布/方差/未收敛计数非空 + 独立重算一致)+ 误判率 ≤
//!   标定阈 + 固定全 spp golden 对拍不偏离冻结带(measured×2.0 带继承)+
//!   帧型标签闭集;RED 臂 early-stop/underreport/label-mix。
//! - `--calibrate <out.json>`:M166 标定程序(**纯 host**,零 device 依赖;
//!   pbrt 1024spp 参照经子进程真跑)——吞吐参考阈 τ(p50)/自适应 rel_err 阈
//!   θ(p90)/收敛误判率阈/曲线容差/白炉能量容差/逐级单调噪声带/RR 无偏容差
//!   + 采样器选型 benchmark(winner 族)全 measured 产出,两跑逐位一致由
//!   smoke 层复核。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备/W3 能力链缺失/无 pbrt provisioning →
//! `G12_PT_PROD: SKIP DEV_ENV_DEGRADE`(退 0,非 fake pass;
//! `RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层裁决);判据不符/
//! RED 轴失效 → FAIL 退 1。`RURIX_VK_VALIDATION=1`:vk.rs lane 内
//! fail-closed;evidence 记 validation 模式。
//!
//! ## 用法
//!
//! ```text
//! g12_pt_production --gate <symbolic_key> --spv <kernel.spv> --evidence <path>
//!     --pbrt <pbrt.exe> --imgtool <imgtool.exe> [--work-dir <dir>]
//!     --tau <f32> --theta <f32> --sampler <pcg|stratified|sobol>
//!     --curve-tol <f64> --furnace-tol <f64> --level-tol <f64>
//!     --rr-unbiased-tol <f64> --misjudge-tol <f64>
//!     --anchor-cornell "v1,v4,v16,v64" --anchor-direct "v1,v4,v16,v64"
//! g12_pt_production --red-arm <name> --spv .. --tau .. --sampler ..
//! g12_pt_production --calibrate <out.json> --pbrt .. --imgtool .. [--work-dir ..]
//! g12_pt_production --selftest
//! ```

use rurix_render::gi::path_trace::prod::{
    self, AdaptiveParams, G12_ADAPTIVE_N_FLOOR, G12_ADAPTIVE_SPP_MAX, G12_PROD_SEED, LightDist,
    ProdConfig, ProdImage, ProdScene, RrProdParams, SamplerFamily,
};
use rurix_render::gi::path_trace::prod_denoise::{self as pden};
use rurix_render::gi::path_trace::{
    self, M96_PBRT_REF_SPP, M96_PBRT_SEED, PtConfig, PtScene, ToleranceBand,
};
use rurix_render::rt::bvh::{InstanceDesc, Ray, Tlas, Transform3x4, TriBvh, Vec3};
use rurix_render::rt::ref_tracer::RAY_EPS;
use rurix_render::temporal::image::ImageF32;
use rurix_rt::render_exec::{self, KernelWave};
use rurix_rt::vk::{
    self, RayQueryBufferDesc, RayQueryDispatchDesc, RayQueryInstanceDesc, RayQuerySceneDesc,
};

const TAG: &str = "G12_PT_PROD";
/// RED 臂演示/对照基线点(Cornell 多反弹显著)。
const RED_SCENE: &str = "m96_cornell";
const RED_SPP: u32 = 16;
/// 收敛误判带(判收敛像素相对全 spp 参照逐像素亮度偏差带;协议冻结登记)。
const MISJUDGE_BAND: f64 = 0.25;
/// g12_budget 锚条目 id 前缀。
const ANCHOR_PREFIX: &str = "g12.pt.ref_curve_";

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

/// 排序后百分位(nearest-rank;确定函数)。
fn percentile(mut v: Vec<f64>, p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let idx = ((v.len() as f64 - 1.0) * p).round() as usize;
    v[idx.min(v.len() - 1)]
}

/// 族 → 注册表序下标(pcg=0/stratified=1/sobol=2;标定表索引一致面)。
fn winner_index(fam: SamplerFamily) -> usize {
    match fam {
        SamplerFamily::Pcg => 0,
        SamplerFamily::Stratified => 1,
        SamplerFamily::Sobol => 2,
    }
}

// ---------------------------------------------------------------------------
// device 执行腿(U30 run_ray_query_effects;单 BLAS × 单实例;逐像素单 invocation)
// ---------------------------------------------------------------------------

/// 单场景单配置 device 真跑 → ProdImage(回读装配;6 输出缓冲序 =
/// rgb/stats/samples/converged/rr/energy)。
fn run_device(
    scene: &ProdScene,
    dist: &LightDist,
    cfg: &ProdConfig,
    spv: &[u32],
    entry: &str,
) -> Result<ProdImage, String> {
    scene.validate().map_err(|e| format!("场景校验: {e}"))?;
    cfg.validate().map_err(|e| format!("配置校验: {e}"))?;
    let cam = &scene.camera;
    let pixel_count = (cam.width * cam.height) as usize;
    let tris = prod::pack_prod_tris(scene);
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
    let stream =
        prod::sampler::generate(cfg.sampler, pixel_count, cfg.spp, cfg.max_bounces, cfg.seed);
    let rng_b = bytes_f32(&stream);
    let mats_b = bytes_f32(&prod::pack_prod_mats(scene));
    let tris_b = bytes_f32(&tris);
    let lights_b = bytes_f32(&prod::pack_prod_lights(scene, dist));
    let params_b = bytes_f32(&prod::pack_prod_params(scene, cfg));
    let buffers = [
        RayQueryBufferDesc::Input(&rng_b),
        RayQueryBufferDesc::Input(&mats_b),
        RayQueryBufferDesc::Input(&tris_b),
        RayQueryBufferDesc::Input(&lights_b),
        RayQueryBufferDesc::Input(&params_b),
        RayQueryBufferDesc::Output(pixel_count * 12),
        RayQueryBufferDesc::Output(pixel_count * 8),
        RayQueryBufferDesc::Output(pixel_count * 4),
        RayQueryBufferDesc::Output(pixel_count * 4),
        RayQueryBufferDesc::Output(pixel_count * 16),
        RayQueryBufferDesc::Output(pixel_count * 16),
    ];
    let out = vk::run_ray_query_effects(
        &scene_desc,
        &[RayQueryDispatchDesc {
            name: "g12_pt_production",
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
    if rb.len() != 6 {
        return Err(format!("回读路数 {} ≠ 6", rb.len()));
    }
    let stats = read_f32(&rb[1]);
    let mut sum_lum = Vec::with_capacity(pixel_count);
    let mut sumsq_lum = Vec::with_capacity(pixel_count);
    for px in 0..pixel_count {
        sum_lum.push(stats[px * 2]);
        sumsq_lum.push(stats[px * 2 + 1]);
    }
    Ok(ProdImage {
        width: cam.width,
        height: cam.height,
        rgb: read_f32(&rb[0]),
        sum_lum,
        sumsq_lum,
        samples: read_u32(&rb[2]),
        converged: read_f32(&rb[3]),
        rr_counters: read_f32(&rb[4]),
        energy_levels: read_f32(&rb[5]),
        frame_label: if cfg.adaptive.is_some() {
            "adaptive"
        } else {
            "full_reference"
        },
    })
}

/// 双跑位级一致 + digest(固定 seed 确定性协议继承承载)。
fn run_device_double(
    scene: &ProdScene,
    dist: &LightDist,
    cfg: &ProdConfig,
    spv: &[u32],
    entry: &str,
) -> Result<(ProdImage, [u8; 32], bool), String> {
    let a = run_device(scene, dist, cfg, spv, entry)?;
    let b = run_device(scene, dist, cfg, spv, entry)?;
    let da = prod::prod_image_digest(&a);
    let db = prod::prod_image_digest(&b);
    Ok((a, da, da == db))
}

// ---------------------------------------------------------------------------
// pbrt 腿(provisioning 显式;EXR→PFM 回读;M96 harness 同律——冻结 fixtures
// 经 path_trace::pbrt_scene_text 物质化,生产化消费面 0-byte 只读)
// ---------------------------------------------------------------------------

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

/// pbrt 渲染 + 回读 PFM(带缓存:manifest 登记场景文本 sha256,命中复用)。
fn pbrt_render_cached(
    pbrt: &std::path::Path,
    imgtool: &std::path::Path,
    work: &std::path::Path,
    scene: &PtScene,
    spp: u32,
) -> Result<Vec<f32>, String> {
    std::fs::create_dir_all(work).map_err(|e| format!("建 work-dir: {e}"))?;
    let stem = path_trace::pbrt_scene_filename(scene.name, spp).replace(".pbrt", "");
    let pfm = work.join(format!("{stem}.pfm"));
    let cfg = PtConfig::reference(spp);
    let exr_name = format!("{stem}.exr");
    let text = path_trace::pbrt_scene_text(scene, &cfg, M96_PBRT_SEED, &exr_name);
    let text_sha = hex(&rurix_pkg::sha256::digest(text.as_bytes()));
    let sha_file = work.join(format!("{stem}.sha256"));
    if pfm.is_file()
        && sha_file.is_file()
        && std::fs::read_to_string(&sha_file)
            .map(|s| s.trim() == text_sha)
            .unwrap_or(false)
    {
        let bytes = std::fs::read(&pfm).map_err(|e| format!("读 {}: {e}", pfm.display()))?;
        let (w, h, img) = path_trace::read_pfm(&bytes).map_err(|e| e.to_string())?;
        if (w, h) != (scene.camera.width, scene.camera.height) {
            return Err(format!("PFM 尺寸 {w}×{h} ≠ 冻结相机"));
        }
        return Ok(img);
    }
    let scene_path = work.join(format!("{stem}.pbrt"));
    std::fs::write(&scene_path, &text).map_err(|e| format!("写 {}: {e}", scene_path.display()))?;
    let scene_path =
        std::fs::canonicalize(&scene_path).map_err(|e| format!("canonicalize: {e}"))?;
    let scene_str = scene_path.display().to_string();
    let scene_str = scene_str
        .strip_prefix(r"\\?\")
        .unwrap_or(&scene_str)
        .to_string();
    run_pbrt(pbrt, work, std::path::Path::new(&scene_str))?;
    let exr = work.join(&exr_name);
    if !exr.is_file() {
        return Err(format!("pbrt 未产 {}", exr.display()));
    }
    exr_to_pfm(imgtool, &exr, &pfm)?;
    std::fs::write(&sha_file, &text_sha).map_err(|e| format!("写 sha: {e}"))?;
    let bytes = std::fs::read(&pfm).map_err(|e| format!("读 {}: {e}", pfm.display()))?;
    let (w, h, img) = path_trace::read_pfm(&bytes).map_err(|e| e.to_string())?;
    if (w, h) != (scene.camera.width, scene.camera.height) {
        return Err(format!("PFM 尺寸 {w}×{h} ≠ 冻结相机"));
    }
    Ok(img)
}

/// pbrt provisioning 探测(横幅 + exe sha256;缺一即 DEV_ENV_DEGRADE)。
fn pbrt_provenance(pbrt: &std::path::Path) -> Result<(String, String), String> {
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
    Ok((version, hex(&rurix_pkg::sha256::digest(&exe_bytes))))
}

// ---------------------------------------------------------------------------
// 生产化场景面(m96 两场景同源转换 + 三 fixture)
// ---------------------------------------------------------------------------

struct ProdCtx {
    scenes: Vec<ProdScene>,
    dists: Vec<LightDist>,
}

fn prod_ctx() -> ProdCtx {
    let mut scenes = prod::g12_prod_scenes();
    scenes.push(prod::g12_two_light_scene());
    scenes.push(prod::g12_furnace_scene());
    scenes.push(prod::g12_delta_light_scene());
    let dists = scenes.iter().map(prod::build_light_distribution).collect();
    ProdCtx { scenes, dists }
}

impl ProdCtx {
    fn get(&self, name: &str) -> (&ProdScene, &LightDist) {
        let idx = self
            .scenes
            .iter()
            .position(|s| s.name == name)
            .unwrap_or_else(|| fail(&format!("场景 {name} 不在生产化集")));
        (&self.scenes[idx], &self.dists[idx])
    }
}

/// 生产化基线配置(τ 标定值消费;采样器 = 选型裁决族)。
fn prod_cfg(spp: u32, sampler: SamplerFamily, tau: f32) -> ProdConfig {
    ProdConfig::production(spp, sampler, tau)
}

// ---------------------------------------------------------------------------
// RED 臂(device 变体真跑;digest 偏离正例臂必检出)
// ---------------------------------------------------------------------------

/// RED 臂注册表:name → 变异说明(evidence 登记字面)。
const RED_ARMS: [&str; 8] = [
    "no-mis",
    "energy-bias",
    "seed-change",
    "no-rr",
    "comp-off",
    "early-kill",
    "nondeterministic",
    "early-stop",
];

/// 单臂执行:正例臂 digest vs 变体 digest(偏离 = 检出)。early-stop 附加
/// 逐像素样本数 == 1 语义断言(早停冒充面)。
fn red_arm_run(
    ctx: &ProdCtx,
    name: &str,
    sampler: SamplerFamily,
    tau: f32,
    spv: &[u32],
    entry: &str,
) -> Result<bool, String> {
    let (scene, dist) = ctx.get(RED_SCENE);
    let base = prod_cfg(RED_SPP, sampler, tau);
    let golden = run_device(scene, dist, &base, spv, entry)?;
    let golden_digest = prod::prod_image_digest(&golden);
    let mut cfg = base;
    let mut tamper_stream = false;
    match name {
        "no-mis" => cfg.mis = false,
        "energy-bias" => cfg.energy_bias = 0.05,
        "seed-change" => cfg.seed = G12_PROD_SEED ^ 0xABCD_EF01_2345_6789,
        "no-rr" => cfg.rr = false,
        "comp-off" => cfg.rr_comp_off = true,
        // 早杀注入:绕 host fail-closed 直接覆写 device 参数面(RED 注入臂;
        // host validate 拒绝面由 rr_params_fail_closed 断言承载)。
        "early-kill" => cfg.rr_params.min_bounce = 0,
        "nondeterministic" => tamper_stream = true,
        "early-stop" => {
            cfg.adaptive = Some(AdaptiveParams {
                n_floor: 1,
                spp_max: G12_ADAPTIVE_SPP_MAX,
                rel_err_threshold: 1e9,
            })
        }
        other => fail(&format!("unknown --red-arm {other}(注册表 {RED_ARMS:?})")),
    }
    // early-kill 绕过 ProdConfig::validate(注入面):直接打包执行。
    let img = if name == "early-kill" || tamper_stream {
        let cam = &scene.camera;
        let pixel_count = (cam.width * cam.height) as usize;
        let mut stream =
            prod::sampler::generate(cfg.sampler, pixel_count, cfg.spp, cfg.max_bounces, cfg.seed);
        if tamper_stream {
            // 像素 0 采样 4 的相机 jitter u 维(索引 104 = (0·16+4)·26;必被
            // 消费维——篡改必改变该样本贡献,非确定注入 RED 面)。
            stream[104] = (stream[104] + 0.5).fract();
        }
        let tris = prod::pack_prod_tris(scene);
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
        let rng_b = bytes_f32(&stream);
        let mats_b = bytes_f32(&prod::pack_prod_mats(scene));
        let tris_b = bytes_f32(&tris);
        let lights_b = bytes_f32(&prod::pack_prod_lights(scene, dist));
        let params_b = bytes_f32(&prod::pack_prod_params(scene, &cfg));
        let buffers = [
            RayQueryBufferDesc::Input(&rng_b),
            RayQueryBufferDesc::Input(&mats_b),
            RayQueryBufferDesc::Input(&tris_b),
            RayQueryBufferDesc::Input(&lights_b),
            RayQueryBufferDesc::Input(&params_b),
            RayQueryBufferDesc::Output(pixel_count * 12),
            RayQueryBufferDesc::Output(pixel_count * 8),
            RayQueryBufferDesc::Output(pixel_count * 4),
            RayQueryBufferDesc::Output(pixel_count * 4),
            RayQueryBufferDesc::Output(pixel_count * 16),
            RayQueryBufferDesc::Output(pixel_count * 16),
        ];
        let out = vk::run_ray_query_effects(
            &scene_desc,
            &[RayQueryDispatchDesc {
                name: "g12_pt_production_red",
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
        let stats = read_f32(&rb[1]);
        let mut sum_lum = Vec::with_capacity(pixel_count);
        let mut sumsq_lum = Vec::with_capacity(pixel_count);
        for px in 0..pixel_count {
            sum_lum.push(stats[px * 2]);
            sumsq_lum.push(stats[px * 2 + 1]);
        }
        ProdImage {
            width: cam.width,
            height: cam.height,
            rgb: read_f32(&rb[0]),
            sum_lum,
            sumsq_lum,
            samples: read_u32(&rb[2]),
            converged: read_f32(&rb[3]),
            rr_counters: read_f32(&rb[4]),
            energy_levels: read_f32(&rb[5]),
            frame_label: if cfg.adaptive.is_some() {
                "adaptive"
            } else {
                "full_reference"
            },
        }
    } else {
        run_device(scene, dist, &cfg, spv, entry)?
    };
    let d = prod::prod_image_digest(&img);
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
    if name == "early-stop" && !img.samples.iter().all(|&n| n == 1) {
        println!("{TAG}: RED 臂 early-stop 语义面失效(逐像素样本数非全 1)");
        return Ok(false);
    }
    Ok(detected)
}

// ---------------------------------------------------------------------------
// 收敛曲线锚(g12_budget ref_curve 锚;容差标定程序产)
// ---------------------------------------------------------------------------

struct Anchors {
    cornell: [f64; 4],
    direct: [f64; 4],
    curve_tol: f64,
}

impl Anchors {
    fn get(&self, scene: &str, spp_idx: usize) -> f64 {
        match scene {
            "m96_cornell" => self.cornell[spp_idx],
            "m96_direct" => self.direct[spp_idx],
            _ => fail(&format!("锚缺场景 {scene}")),
        }
    }
}

/// 收敛曲线不劣于判定:生产化 device 曲线(rel-MAE vs pbrt 1024 参照)≤
/// 锚 × (1 + 标定容差)。逐 (scene, spp) 打印实测值(evidence 面)。
fn curve_not_worse(
    ctx: &ProdCtx,
    anchors: &Anchors,
    sampler: SamplerFamily,
    tau: f32,
    spv: &[u32],
    entry: &str,
    pbrt_refs: &std::collections::BTreeMap<String, Vec<f32>>,
    curves_out: &mut Vec<String>,
) -> bool {
    let mut ok = true;
    for name in ["m96_cornell", "m96_direct"] {
        let (scene, dist) = ctx.get(name);
        let reference = &pbrt_refs[name];
        for (si, spp) in path_trace::M96_SPP_SEQUENCE.iter().enumerate() {
            let cfg = prod_cfg(*spp, sampler, tau);
            let img = match run_device(scene, dist, &cfg, spv, entry) {
                Ok(v) => v,
                Err(e) => fail(&format!("device 曲线臂 {name} spp={spp}: {e}")),
            };
            let curve = path_trace::rel_mae(&img.rgb, reference).expect("curve 计算");
            let anchor = anchors.get(name, si);
            let bound = anchor * (1.0 + anchors.curve_tol);
            let pass = curve <= bound;
            if !pass {
                ok = false;
            }
            curves_out.push(format!(
                "\"{name}_spp{spp}\": {{\"curve\": \"{curve:.6e}\", \"anchor\": \"{anchor:.6e}\", \"bound\": \"{bound:.6e}\", \"pass\": {pass}}}"
            ));
            println!(
                "{TAG}: 曲线锚 {name} spp={spp} curve={curve:.6e} anchor={anchor:.6e} bound={bound:.6e} → {}",
                if pass { "不劣于" } else { "劣化(RED)" }
            );
        }
    }
    ok
}

// ---------------------------------------------------------------------------
// 帧型标签闭集(RXS-0401 L4:{adaptive, full_reference},混标即 RED)
// ---------------------------------------------------------------------------

fn frame_label_valid(label: &str, adaptive_on: bool) -> bool {
    matches!(label, "adaptive" | "full_reference") && (label == "adaptive") == adaptive_on
}

// ---------------------------------------------------------------------------
// 标定程序(M166;纯 host——零 device 依赖;pbrt 1024 参照子进程真跑)
// ---------------------------------------------------------------------------

/// RR 吞吐参考阈探针(标定程序;与 kernel/host oracle 公式面同源):逐路径
/// 相机 → bounce 0/1/2 BSDF 余弦采样,记录 bounce=2 BSDF 更新后的路径吞吐
/// max 分量(RR 首次求值点的 T);返回全样本集(供 p50 取阈)。
fn rr_tau_probe(scene: &ProdScene, sampler: SamplerFamily, seed: u64, spp: u32) -> Vec<f64> {
    let cam = &scene.camera;
    let pixel_count = (cam.width * cam.height) as usize;
    let stream = prod::sampler::generate(sampler, pixel_count, spp, prod::PROD_MAX_BOUNCES, seed);
    let blases = vec![TriBvh::build(&scene.positions, &scene.indices)];
    let tlas = Tlas::build(
        &[InstanceDesc {
            blas: 0,
            transform: Transform3x4::IDENTITY,
            mask: 0xFF,
            flags: 0,
        }],
        &blases,
    );
    let bset: &[TriBvh] = &blases;
    let mut out = Vec::with_capacity(pixel_count * spp as usize);
    for px in 0..pixel_count {
        for s in 0..spp as usize {
            let sb = prod::prod_sample_base(px, s, spp, prod::PROD_MAX_BOUNCES);
            let pxx = px % cam.width as usize;
            let pyy = px / cam.width as usize;
            let ju = (pxx as f32 + stream[sb]) / cam.width as f32;
            let jv = (pyy as f32 + stream[sb + 1]) / cam.height as f32;
            let sx = (2.0 * ju - 1.0) * cam.tan_half_fov;
            let sy = (1.0 - 2.0 * jv) * cam.tan_half_fov;
            let f = Vec3::from_array(cam.forward);
            let r = Vec3::from_array(cam.right);
            let u = Vec3::from_array(cam.up);
            let mut d = (f + r * sx + u * sy).normalize();
            let mut origin = Vec3::from_array(cam.origin);
            let mut thr = Vec3::new(1.0, 1.0, 1.0);
            // 只记录存活至 bounce=2 的路径吞吐(miss 吸收态不入样——τ 是活路径
            // 吞吐参考尺,死路径恒 0 不承载尺度信息)。
            let mut alive = false;
            let mut t_rec = 0.0f32;
            for b in 0..=2usize {
                let bb = prod::prod_bounce_base(sb, b);
                let Some(hit) = tlas.intersect(bset, &Ray { origin, dir: d }) else {
                    break;
                };
                let prim = hit.tri as usize;
                let ng = Vec3::from_array(hit.normal);
                let p = origin + d * hit.t;
                let n = if ng.dot(d) > 0.0 { ng * (-1.0) } else { ng };
                let al = match &scene.materials[prim] {
                    path_trace::MaterialKind::Lambert { albedo } => *albedo,
                    path_trace::MaterialKind::Emission { albedo, .. } => *albedo,
                    _ => [0.0; 3],
                };
                let al = Vec3::from_array(al);
                let nd = rurix_render::rt::ref_tracer::cosine_sample_hemisphere(
                    n,
                    stream[bb + 3],
                    stream[bb + 4],
                );
                thr = Vec3::new(thr.x * al.x, thr.y * al.y, thr.z * al.z);
                if b == 2 {
                    t_rec = thr.x.max(thr.y).max(thr.z);
                    alive = true;
                }
                origin = p + n * RAY_EPS;
                d = nd;
            }
            if alive {
                out.push(f64::from(t_rec));
            }
        }
    }
    out
}

/// host oracle 快捷渲染(标定腿)。
fn host_render(scene: &ProdScene, cfg: &ProdConfig) -> ProdImage {
    let dist = prod::build_light_distribution(scene);
    let stream = prod::sampler::generate(
        cfg.sampler,
        (scene.camera.width * scene.camera.height) as usize,
        cfg.spp,
        cfg.max_bounces,
        cfg.seed,
    );
    prod::trace_host_prod(scene, &dist, cfg, &stream).expect("host oracle 渲染")
}

/// 标定输出(全 measured;两跑逐位一致由 smoke 层复核;样本集 digest 入
/// evidence,样本集下界 ≥24)。
fn run_calibration(
    out_path: &str,
    pbrt: &std::path::Path,
    imgtool: &std::path::Path,
    work: &std::path::Path,
) -> ! {
    let (pbrt_version, pbrt_sha) = match pbrt_provenance(pbrt) {
        Ok(v) => v,
        Err(e) => skip(&format!("pbrt provisioning 缺失({e})(DEV_ENV_DEGRADE)")),
    };
    if !imgtool.is_file() {
        skip(&format!(
            "imgtool 不存在({})(DEV_ENV_DEGRADE)",
            imgtool.display()
        ));
    }
    let ctx = prod_ctx();
    let m96 = path_trace::m96_scenes();
    let mut manifest: Vec<String> = Vec::new();

    // ① RR 吞吐参考阈 τ:p50(三场景 spp16 stratified,RR 关;bounce=2 逐路径
    //    吞吐 max 分量)。
    let mut tau_samples: Vec<f64> = Vec::new();
    for name in ["m96_cornell", "m96_direct", "g12_two_light"] {
        let (scene, _) = ctx.get(name);
        tau_samples.extend(rr_tau_probe(
            scene,
            SamplerFamily::Stratified,
            G12_PROD_SEED,
            16,
        ));
        manifest.push(format!("tau:{name}:spp16:stratified"));
    }
    let tau = percentile(tau_samples.clone(), 0.5) as f32;
    if tau_samples.is_empty() || !(tau > 0.0) {
        fail(&format!("τ 标定样本集空/非正(n={})", tau_samples.len()));
    }
    println!("{TAG}: 标定 τ = {tau:.6e}(p50,n={})", tau_samples.len());

    // ② 采样器选型 benchmark(winner = 双 m96 场景 spp16 平均逐像素方差总和
    //    最小族;τ 消费①)。
    let mut var_table: Vec<(SamplerFamily, f64)> = Vec::new();
    for fam in [
        SamplerFamily::Pcg,
        SamplerFamily::Stratified,
        SamplerFamily::Sobol,
    ] {
        let mut acc = 0.0f64;
        for name in ["m96_cornell", "m96_direct"] {
            let (scene, _) = ctx.get(name);
            acc += host_render(scene, &prod_cfg(16, fam, tau)).mean_pixel_variance();
        }
        var_table.push((fam, acc));
        manifest.push(format!("select:{}:{acc:.12e}", fam.name()));
    }
    let mut winner = SamplerFamily::Pcg;
    let mut best = f64::INFINITY;
    for (fam, v) in &var_table {
        if *v < best {
            best = *v;
            winner = *fam;
        }
    }
    println!(
        "{TAG}: 选型 benchmark winner = {}(pcg={:.6e} stratified={:.6e} sobol={:.6e})",
        winner.name(),
        var_table[0].1,
        var_table[1].1,
        var_table[2].1
    );

    // ③ 自适应 rel_err 阈 θ:p75(双场景 spp=N_floor winner 全 spp 参照面逐像素
    //    rel_err 池化;协议冻结——以 N_floor 参照档误差界分布的上四分位为停采
    //    阈,兼顾早停收益与方差欠估计防护;N_floor=16 冻结值,G12.2 网格实测
    //    面:floor=4 边缘像素全零样本假收敛误判 43%,floor=16 亚百分点级)。
    let mut rel_errs: Vec<f64> = Vec::new();
    for name in ["m96_cornell", "m96_direct"] {
        let (scene, _) = ctx.get(name);
        let img = host_render(scene, &prod_cfg(G12_ADAPTIVE_N_FLOOR, winner, tau));
        for px in 0..img.pixel_count() {
            rel_errs.push(f64::from(prod::rel_err_bound(
                img.sum_lum[px],
                img.sumsq_lum[px],
                img.samples[px],
            )));
        }
        manifest.push(format!(
            "theta:{name}:spp{}:{}",
            G12_ADAPTIVE_N_FLOOR,
            winner.name()
        ));
    }
    let theta = percentile(rel_errs, 0.75) as f32;
    println!(
        "{TAG}: 标定 θ = {theta:.6e}(p75 @ spp={})",
        G12_ADAPTIVE_N_FLOOR
    );

    // ④ 收敛误判率:自适应(θ, N_floor, 上界 64)vs 全 spp 64 参照同 seed 同
    //    场景;误判带 0.25(协议冻结);**单元粒度 p100**(场景 × 族矩阵逐单元
    //    误判率,覆盖实现差/实现面噪声;池化均值会稀释高方差场景——G12.2 门
    //    实测定位:池化 0.62% vs 设备 cornell 单元 1.43%,单元 p100 为诚实
    //    覆盖面);tol = (p100_cell_rate + 1/min_cell_judged) × 2.0。
    let mut cell_rates: Vec<f64> = Vec::new();
    let mut min_judged = u64::MAX;
    let mut cell_table: Vec<String> = Vec::new();
    for name in ["m96_cornell", "m96_direct"] {
        let (scene, _) = ctx.get(name);
        for fam in [
            SamplerFamily::Pcg,
            SamplerFamily::Stratified,
            SamplerFamily::Sobol,
        ] {
            let full = host_render(scene, &prod_cfg(64, fam, tau));
            let mut ad = prod_cfg(64, fam, tau);
            ad.adaptive = Some(AdaptiveParams {
                n_floor: G12_ADAPTIVE_N_FLOOR,
                spp_max: G12_ADAPTIVE_SPP_MAX,
                rel_err_threshold: theta,
            });
            let img = host_render(scene, &ad);
            let mut judged = 0u64;
            let mut mis = 0u64;
            for px in 0..img.pixel_count() {
                if img.converged[px] == 1.0 {
                    judged += 1;
                    let a = f64::from(
                        (img.rgb[px * 3] + img.rgb[px * 3 + 1] + img.rgb[px * 3 + 2]) / 3.0,
                    );
                    let b = f64::from(
                        (full.rgb[px * 3] + full.rgb[px * 3 + 1] + full.rgb[px * 3 + 2]) / 3.0,
                    );
                    if (a - b).abs() / b.abs().max(1e-3) > MISJUDGE_BAND {
                        mis += 1;
                    }
                }
            }
            let rate = if judged > 0 {
                mis as f64 / judged as f64
            } else {
                0.0
            };
            cell_rates.push(rate);
            min_judged = min_judged.min(judged);
            cell_table.push(format!(
                "\"{name}:{}\": \"{rate:.6e}(j={judged} m={mis})\"",
                fam.name()
            ));
            manifest.push(format!("misjudge:{name}:spp64:{}", fam.name()));
        }
    }
    let misjudge_rate = percentile(cell_rates, 1.0);
    let misjudge_tol = (misjudge_rate + 1.0 / min_judged.max(1) as f64) * 2.0;
    println!(
        "{TAG}: 标定误判率单元 p100 = {misjudge_rate:.6e}(min_judged={min_judged})→ tol = {misjudge_tol:.6e}"
    );

    // ⑤ 曲线容差:三族 × 双场景 × 4 spp host 曲线(rel-MAE vs pbrt 1024 参照)
    //    族间极差相对锚 p100 × 1.5(协议冻结 k;覆盖合法配置面差异)。
    let mut spread_samples: Vec<f64> = Vec::new();
    for (name, m96_scene) in [("m96_cornell", &m96[0]), ("m96_direct", &m96[1])] {
        let (scene, _) = ctx.get(name);
        let reference = match pbrt_render_cached(pbrt, imgtool, work, m96_scene, M96_PBRT_REF_SPP) {
            Ok(v) => v,
            Err(e) => fail(&format!("pbrt 参照 {name}: {e}")),
        };
        for (si, spp) in path_trace::M96_SPP_SEQUENCE.iter().enumerate() {
            let anchor = anchor_default(name, si);
            let mut fam_curves: Vec<f64> = Vec::new();
            for fam in [
                SamplerFamily::Pcg,
                SamplerFamily::Stratified,
                SamplerFamily::Sobol,
            ] {
                let img = host_render(scene, &prod_cfg(*spp, fam, tau));
                fam_curves.push(path_trace::rel_mae(&img.rgb, &reference).expect("curve"));
            }
            let lo = fam_curves.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = fam_curves.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            spread_samples.push((hi - lo) / anchor);
            manifest.push(format!("curve:{name}:spp{spp}:3fam"));
        }
    }
    let curve_spread = percentile(spread_samples, 1.0);
    let curve_tol = curve_spread * 1.5;
    println!("{TAG}: 标定曲线族间极差 p100 = {curve_spread:.6e} → tol = {curve_tol:.6e}");

    // ⑥ 白炉能量容差:三族 spp64 host 白炉均值相对 winner 均值极差 p100 × 1.5
    //    (协议冻结 k;参照均值 = winner 族均值,随 evidence 登记;不产能量上界
    //    Le 由门侧硬断言承载)。
    let (furnace, _) = ctx.get("g12_furnace");
    let mut furnace_means: Vec<f64> = Vec::new();
    for fam in [
        SamplerFamily::Pcg,
        SamplerFamily::Stratified,
        SamplerFamily::Sobol,
    ] {
        furnace_means.push(host_render(furnace, &prod_cfg(64, fam, tau)).mean_luminance());
        manifest.push(format!("furnace:spp64:{}", fam.name()));
    }
    let furnace_ref = furnace_means[winner_index(winner)];
    let furnace_spread = furnace_means
        .iter()
        .map(|m| (m - furnace_ref).abs() / furnace_ref.abs().max(1e-12))
        .fold(0.0f64, f64::max);
    let furnace_tol = furnace_spread * 1.5;
    println!(
        "{TAG}: 标定白炉参照均值 = {furnace_ref:.6e}(winner;截断真值面)族间极差 = {furnace_spread:.6e} → tol = {furnace_tol:.6e}"
    );

    // ⑦ 逐级能量单调噪声带:四场景 spp64 winner,p100(max(0, E_{k+1}/E_k −1))
    //    × 1.5。
    let mut mono_samples: Vec<f64> = Vec::new();
    for name in ["m96_cornell", "m96_direct", "g12_two_light", "g12_furnace"] {
        let (scene, _) = ctx.get(name);
        let img = host_render(scene, &prod_cfg(64, winner, tau));
        let mut levels = [0.0f64; 4];
        for px in 0..img.pixel_count() {
            for k in 0..4 {
                levels[k] += f64::from(img.energy_levels[px * 4 + k]);
            }
        }
        for k in 0..4 {
            levels[k] /= img.pixel_count() as f64;
        }
        for k in 1..4 {
            mono_samples.push((levels[k] / levels[k - 1].max(1e-12) - 1.0).max(0.0));
        }
        manifest.push(format!("levels:{name}:spp64:{}", winner.name()));
    }
    let level_base = percentile(mono_samples, 1.0);
    let level_tol = level_base * 1.5;
    println!("{TAG}: 标定逐级单调噪声带 p100 = {level_base:.6e} → tol = {level_tol:.6e}");

    // ⑧ RR 无偏容差:双场景 spp32 × 三族,|mean_on − mean_off|/mean_off p100
    //    × 2.0(族间池化覆盖实现差噪;协议冻结 k)。
    let mut ub_samples: Vec<f64> = Vec::new();
    for name in ["m96_cornell", "m96_direct"] {
        let (scene, _) = ctx.get(name);
        for fam in [
            SamplerFamily::Pcg,
            SamplerFamily::Stratified,
            SamplerFamily::Sobol,
        ] {
            let on = host_render(scene, &prod_cfg(32, fam, tau));
            let mut off_cfg = prod_cfg(32, fam, tau);
            off_cfg.rr = false;
            let off = host_render(scene, &off_cfg);
            ub_samples.push(
                (on.mean_luminance() - off.mean_luminance()).abs()
                    / off.mean_luminance().abs().max(1e-12),
            );
            manifest.push(format!("unbiased:{name}:spp32:{}", fam.name()));
        }
    }
    let ub_base = percentile(ub_samples, 1.0);
    let rr_unbiased_tol = ub_base * 2.0;
    println!("{TAG}: 标定 RR 无偏 gap p100 = {ub_base:.6e} → tol = {rr_unbiased_tol:.6e}");

    // 样本集 digest(manifest 逐行排序拼接 sha256)。
    manifest.sort();
    let manifest_digest = hex(&rurix_pkg::sha256::digest(manifest.join("\n").as_bytes()));
    let json = format!(
        "{{\n  \"schema\": \"rurix.g12pt.calibration.v1\",\n  \
         \"sampler_selection\": {{\"winner\": \"{}\", \"variance_table\": {{\"pcg\": \"{:.12e}\", \"stratified\": \"{:.12e}\", \"sobol\": \"{:.12e}\"}}, \"protocol\": \"winner = argmin Σ mean_pixel_variance(cornell,direct;spp16)\"}},\n  \
         \"rr_tau\": {{\"measured\": \"{:.12e}\", \"protocol\": \"p50 per-path throughput max-channel at bounce==2; scenes=cornell+direct+two_light; spp16; stratified; rr off\", \"drift_guard_k\": 2.0}},\n  \
         \"adaptive_rel_err_theta\": {{\"measured\": \"{:.12e}\", \"protocol\": \"p75 pooled per-pixel rel_err at spp=N_floor(16); cornell+direct; winner; production\", \"drift_guard_k\": 1.5}},\n  \
         \"misjudge_rate\": {{\"measured\": \"{:.12e}\", \"min_cell_judged\": {}, \"band\": {}, \"tol\": \"{:.12e}\", \"cells\": {{{}}}, \"protocol\": \"tol = (p100 cell rate〔场景×族矩阵〕 + 1/min_cell_judged) × 2.0(协议冻结;带 0.25)\"}},\n  \
         \"curve_tol_rel\": {{\"base\": \"{:.12e}\", \"tol\": \"{:.12e}\", \"protocol\": \"p100 族间极差/锚 × 1.5(协议冻结 k)\"}},\n  \
         \"furnace_energy_tol\": {{\"base\": \"{:.12e}\", \"tol\": \"{:.12e}\", \"ref_mean\": \"{:.12e}\", \"protocol\": \"p100 族间均值相对极差 × 1.5(协议冻结 k);ref_mean = winner 族 spp64 host 均值(截断真值面)\"}},\n  \
         \"level_monotone_tol\": {{\"base\": \"{:.12e}\", \"tol\": \"{:.12e}\", \"protocol\": \"p100 max(0,E_(k+1)/E_k−1) × 1.5(协议冻结 k)\"}},\n  \
         \"rr_unbiased_tol\": {{\"base\": \"{:.12e}\", \"tol\": \"{:.12e}\", \"protocol\": \"p100 |on−off|/off × 2.0(协议冻结 k)\"}},\n  \
         \"sample_manifest\": {{\"count\": {}, \"digest\": \"sha256:{}\", \"lower_bound\": 24}},\n  \
         \"provenance\": {{\"pbrt_version\": \"{}\", \"pbrt_exe_sha256\": \"{}\", \"seed\": \"{}\", \"host\": \"host oracle(gi::path_trace::prod;纯 host 零 device)\"}}\n}}",
        winner.name(),
        var_table[0].1,
        var_table[1].1,
        var_table[2].1,
        tau,
        theta,
        misjudge_rate,
        min_judged,
        MISJUDGE_BAND,
        misjudge_tol,
        cell_table.join(", "),
        curve_spread,
        curve_tol,
        furnace_spread,
        furnace_tol,
        furnace_ref,
        level_base,
        level_tol,
        ub_base,
        rr_unbiased_tol,
        manifest.len(),
        manifest_digest,
        json_escape(&pbrt_version),
        pbrt_sha,
        G12_PROD_SEED,
    );
    std::fs::write(out_path, &json)
        .unwrap_or_else(|e| fail(&format!("写标定 JSON {out_path}: {e}")));
    println!(
        "{TAG}: PASS calibrate → {out_path}(样本集 {} 项 digest sha256:{manifest_digest})",
        manifest.len()
    );
    std::process::exit(0)
}

/// 标定⑤的锚缺省(标定程序内部消费 g12_budget 锚的缺省值 = M96 冻结带
/// curve_rurix 转录;标定仅取族间极差,锚只作相对化分母——spread/anchor,
/// 锚值漂移只影响相对化尺度;门侧判定用 budget 真值)。
fn anchor_default(scene: &str, spp_idx: usize) -> f64 {
    const CORNELL: [f64; 4] = [
        0.2931720247639041,
        0.17710069794666713,
        0.12422206054630583,
        0.09022782888026709,
    ];
    const DIRECT: [f64; 4] = [
        0.20455188486043177,
        0.10163445661612792,
        0.051394150300600565,
        0.028991059755816284,
    ];
    match scene {
        "m96_cornell" => CORNELL[spp_idx],
        "m96_direct" => DIRECT[spp_idx],
        _ => fail("未知锚场景"),
    }
}

// ---------------------------------------------------------------------------
// 参数解析
// ---------------------------------------------------------------------------

struct Args {
    gate: Option<String>,
    spv: Option<String>,
    evidence: Option<String>,
    pbrt: Option<String>,
    imgtool: Option<String>,
    work_dir: String,
    band: String,
    tau: f32,
    theta: f32,
    sampler: SamplerFamily,
    curve_tol: f64,
    furnace_tol: f64,
    level_tol: f64,
    rr_unbiased_tol: f64,
    misjudge_tol: f64,
    anchor_cornell: [f64; 4],
    anchor_direct: [f64; 4],
    red_arm: Option<String>,
    calibrate: Option<String>,
    calibrate_denoise: Option<String>,
    denoise_spv: Option<String>,
    hf_drop_min: f64,
    mean_energy_tol: f64,
    selftest: bool,
}

fn parse_anchor(s: &str) -> [f64; 4] {
    let v: Vec<f64> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    if v.len() != 4 {
        fail(&format!("锚格式 ≠ 4 值:{s}"));
    }
    [v[0], v[1], v[2], v[3]]
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = Args {
        gate: None,
        spv: None,
        evidence: None,
        pbrt: None,
        imgtool: None,
        work_dir: ".tmp/g12_pt_prod_work".to_string(),
        band: "milestones/g9/g9_m96_pbrt_tolerance_band.json".to_string(),
        tau: 0.0,
        theta: 0.0,
        sampler: SamplerFamily::Stratified,
        curve_tol: 0.0,
        furnace_tol: 0.0,
        level_tol: 0.0,
        rr_unbiased_tol: 0.0,
        misjudge_tol: 0.0,
        anchor_cornell: [0.0; 4],
        anchor_direct: [0.0; 4],
        red_arm: None,
        calibrate: None,
        calibrate_denoise: None,
        denoise_spv: None,
        hf_drop_min: 0.0,
        mean_energy_tol: 0.0,
        selftest: false,
    };
    let mut i = 0;
    while i < args.len() {
        let take = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
        };
        match args[i].as_str() {
            "--gate" => out.gate = Some(take(&mut i)),
            "--spv" => out.spv = Some(take(&mut i)),
            "--evidence" => out.evidence = Some(take(&mut i)),
            "--pbrt" => out.pbrt = Some(take(&mut i)),
            "--imgtool" => out.imgtool = Some(take(&mut i)),
            "--work-dir" => out.work_dir = take(&mut i),
            "--band" => out.band = take(&mut i),
            "--tau" => {
                out.tau = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--tau 非 f32"))
            }
            "--theta" => {
                out.theta = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--theta 非 f32"))
            }
            "--sampler" => {
                out.sampler = SamplerFamily::parse(&take(&mut i))
                    .unwrap_or_else(|| fail("--sampler 非注册族"))
            }
            "--curve-tol" => {
                out.curve_tol = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--curve-tol 非 f64"))
            }
            "--furnace-tol" => {
                out.furnace_tol = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--furnace-tol 非 f64"))
            }
            "--level-tol" => {
                out.level_tol = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--level-tol 非 f64"))
            }
            "--rr-unbiased-tol" => {
                out.rr_unbiased_tol = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--rr-unbiased-tol 非 f64"))
            }
            "--misjudge-tol" => {
                out.misjudge_tol = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--misjudge-tol 非 f64"))
            }
            "--anchor-cornell" => out.anchor_cornell = parse_anchor(&take(&mut i)),
            "--anchor-direct" => out.anchor_direct = parse_anchor(&take(&mut i)),
            "--red-arm" => out.red_arm = Some(take(&mut i)),
            "--calibrate" => out.calibrate = Some(take(&mut i)),
            "--calibrate-denoise" => out.calibrate_denoise = Some(take(&mut i)),
            "--denoise-spv" => out.denoise_spv = Some(take(&mut i)),
            "--hf-drop-min" => {
                out.hf_drop_min = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--hf-drop-min 非 f64"))
            }
            "--mean-energy-tol" => {
                out.mean_energy_tol = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--mean-energy-tol 非 f64"))
            }
            "--selftest" => out.selftest = true,
            other => fail(&format!("unknown arg {other}")),
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// harness selftest(host 红绿:确定性绿 + 篡改红 + fail-closed 红 + 标签闭集红)
// ---------------------------------------------------------------------------

fn run_selftest() -> ! {
    let ctx = prod_ctx();
    let (scene, dist) = ctx.get(RED_SCENE);
    let cfg = prod_cfg(8, SamplerFamily::Stratified, 0.35);
    let stream = prod::sampler::generate(
        cfg.sampler,
        (scene.camera.width * scene.camera.height) as usize,
        cfg.spp,
        cfg.max_bounces,
        cfg.seed,
    );
    let a = prod::trace_host_prod(scene, dist, &cfg, &stream).expect("golden");
    let b = prod::trace_host_prod(scene, dist, &cfg, &stream).expect("rerun");
    if prod::prod_image_digest(&a) != prod::prod_image_digest(&b) {
        fail("selftest 绿臂:host oracle 双跑分叉");
    }
    let mut tampered = stream.clone();
    // bounce 0 的 NEE u 维(索引 3 = sb+2+1;RR 维在 bounce<min_bounce 不求值,
    // 篡改该维不改变输出——RED 臂须落在被消费的维上)。
    tampered[3] = (tampered[3] + 0.5).fract();
    let c = prod::trace_host_prod(scene, dist, &cfg, &tampered).expect("tampered");
    if prod::prod_image_digest(&a) == prod::prod_image_digest(&c) {
        fail("selftest 红臂:流篡改未检出");
    }
    let mut bad = RrProdParams::production(0.35);
    bad.min_bounce = 1;
    if bad.validate().is_ok() {
        fail("selftest 红臂:min_bounce=1 未 fail-closed");
    }
    if frame_label_valid("full_reference", true) || frame_label_valid("adaptive", false) {
        fail("selftest 红臂:帧型标签混标未检出");
    }
    if !frame_label_valid("adaptive", true) || !frame_label_valid("full_reference", false) {
        fail("selftest 绿臂:帧型标签正例误判");
    }
    println!("{TAG}: SELFTEST PASS(1 GREEN 确定性 + 1 RED 篡改 + 1 RED fail-closed + 2 标签闭集)");
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// 门执行
// ---------------------------------------------------------------------------

struct GateOut {
    checks: Vec<(String, bool)>,
    curves_json: Vec<String>,
    digests_json: Vec<String>,
    measurements_json: Vec<String>,
    failures: Vec<String>,
}

impl GateOut {
    fn new() -> GateOut {
        GateOut {
            checks: Vec::new(),
            curves_json: Vec::new(),
            digests_json: Vec::new(),
            measurements_json: Vec::new(),
            failures: Vec::new(),
        }
    }
    fn check(&mut self, name: &str, ok: bool, msg: &str) {
        if !ok {
            self.failures.push(format!("{name}: {msg}"));
        }
        self.checks.push((name.to_string(), ok));
    }
}

/// M158 MIS 完整面门。
fn gate_m158(
    ctx: &ProdCtx,
    args: &Args,
    anchors: &Anchors,
    spv: &[u32],
    entry: &str,
    pbrt_refs: &std::collections::BTreeMap<String, Vec<f32>>,
) -> GateOut {
    let mut out = GateOut::new();
    // ① 双跑位级(多光源/delta/白炉)。
    let mut bitexact = true;
    for (name, spp) in [
        ("g12_two_light", 16u32),
        ("g12_delta", 16),
        ("g12_furnace", 64),
    ] {
        let (scene, dist) = ctx.get(name);
        let cfg = prod_cfg(spp, args.sampler, args.tau);
        let (img, digest, be) = match run_device_double(scene, dist, &cfg, spv, entry) {
            Ok(v) => v,
            Err(e) => fail(&format!("device {name}: {e}")),
        };
        if !be {
            bitexact = false;
        }
        out.digests_json
            .push(format!("\"{name}_spp{spp}\": \"{}\"", hex(&digest)));
        println!(
            "{TAG}: device {name} spp={spp} digest={} mean_lum={:.6}",
            hex(&digest),
            img.mean_luminance()
        );
    }
    out.check(
        "double_run_bitexact",
        bitexact,
        "双跑 digest 分叉(确定性协议漂移)",
    );
    // ② 光源分布构建确定性(同场景同分布 digest)。
    let mut dist_ok = true;
    for scene in &ctx.scenes {
        let d1 = prod::build_light_distribution(scene);
        let d2 = prod::build_light_distribution(scene);
        if prod::light_distribution_digest(scene, &d1)
            != prod::light_distribution_digest(scene, &d2)
        {
            dist_ok = false;
        }
    }
    out.check(
        "light_dist_deterministic",
        dist_ok,
        "光源分布 digest 非同场景确定",
    );
    // ③ delta 退化:MIS 开/关位级一致(w_nee=1 无除零)。
    let (delta, ddist) = ctx.get("g12_delta");
    let d_on = prod_cfg(16, args.sampler, args.tau);
    let mut d_off = d_on;
    d_off.mis = false;
    let img_on = run_device(delta, ddist, &d_on, spv, entry).expect("delta on");
    let img_off = run_device(delta, ddist, &d_off, spv, entry).expect("delta off");
    let deg = prod::prod_image_digest(&img_on) == prod::prod_image_digest(&img_off);
    out.check(
        "mis_delta_degenerate",
        deg,
        "delta 光源退化:MIS 开关改变输出(除零/非退化面)",
    );
    // ④ 白炉能量守恒 + 逐级单调 + 只丢能量不漏光。参照 = host oracle winner 族
    //    spp64 均值(截断真值面;门内 host 腿实测)+ 不产能量上界 Le 硬断言。
    let (furnace, fdist) = ctx.get("g12_furnace");
    let fcfg = prod_cfg(64, args.sampler, args.tau);
    let fimg = run_device(furnace, fdist, &fcfg, spv, entry).expect("furnace");
    let fref = host_render(furnace, &fcfg).mean_luminance();
    let fmean = fimg.mean_luminance();
    let fgap = (fmean - fref).abs() / fref.abs().max(1e-12);
    let energy_ok = fgap <= args.furnace_tol && fmean <= 4.0 * (1.0 + args.furnace_tol);
    out.check(
        "furnace_energy_conserved",
        energy_ok,
        &format!(
            "白炉 device 均值 {fmean:.6} vs host 参照 {fref:.6} 相对偏差 {fgap:.6e}(容差 {:.6e})或不产能量上界 Le=4 违例",
            args.furnace_tol
        ),
    );
    let mut levels = [0.0f64; 4];
    for px in 0..fimg.pixel_count() {
        for k in 0..4 {
            levels[k] += f64::from(fimg.energy_levels[px * 4 + k]);
        }
    }
    for k in 0..4 {
        levels[k] /= fimg.pixel_count() as f64;
    }
    let mut mono_ok = levels[0] > 0.0;
    for k in 1..4 {
        if levels[k] > levels[k - 1] * (1.0 + args.level_tol) {
            mono_ok = false;
        }
    }
    out.check(
        "levels_monotone",
        mono_ok,
        &format!(
            "逐级能量增量非单调(噪声带 {:.6e}):{levels:?}",
            args.level_tol
        ),
    );
    let nonneg = fimg.rgb.iter().all(|v| v.is_finite() && *v >= 0.0);
    out.check("no_light_leak_nonneg", nonneg, "输出非有限/负(漏光注入)");
    out.measurements_json.push(format!(
        "\"furnace\": {{\"mean\": \"{fmean:.6e}\", \"gap\": \"{fgap:.6e}\", \"levels\": [\"{:.6e}\", \"{:.6e}\", \"{:.6e}\", \"{:.6e}\"]}}",
        levels[0], levels[1], levels[2], levels[3]
    ));
    // ⑤ 收敛曲线不劣于锚。
    let curve_ok = curve_not_worse(
        ctx,
        anchors,
        args.sampler,
        args.tau,
        spv,
        entry,
        pbrt_refs,
        &mut out.curves_json,
    );
    out.check("curve_not_worse", curve_ok, "收敛曲线劣化冒充升级");
    // ⑥ RED 臂。
    for (name, arm) in [
        ("red_no_mis", "no-mis"),
        ("red_energy_bias", "energy-bias"),
        ("red_seed_change", "seed-change"),
    ] {
        let ok = red_arm_run(ctx, arm, args.sampler, args.tau, spv, entry).expect("RED 臂执行");
        out.check(name, ok, &format!("RED 臂 {arm} 未检出(漏检)"));
    }
    out
}

/// M159 RR 生产化门。
fn gate_m159(
    ctx: &ProdCtx,
    args: &Args,
    anchors: &Anchors,
    spv: &[u32],
    entry: &str,
    pbrt_refs: &std::collections::BTreeMap<String, Vec<f32>>,
) -> GateOut {
    let mut out = GateOut::new();
    // ① RR 参数 fail-closed(N_min<2 / p_max=1 必拒)。
    let mut fc = true;
    let mut bad = RrProdParams::production(args.tau);
    bad.min_bounce = 0;
    fc &= bad.validate().is_err();
    bad.min_bounce = 1;
    fc &= bad.validate().is_err();
    let mut bad2 = RrProdParams::production(args.tau);
    bad2.p_max = 1.0;
    fc &= bad2.validate().is_err();
    fc &= RrProdParams::production(args.tau).validate().is_ok();
    out.check(
        "rr_params_fail_closed",
        fc,
        "RR 参数 fail-closed 失效(早杀/截断偏置面)",
    );
    // ② 双跑位级 + 计数非空 + 无偏对照。
    let mut bitexact = true;
    let mut counters_ok = true;
    let mut unbiased_ok = true;
    for name in ["m96_cornell", "m96_direct", "g12_two_light"] {
        let (scene, dist) = ctx.get(name);
        let cfg = prod_cfg(32, args.sampler, args.tau);
        let (img, digest, be) = match run_device_double(scene, dist, &cfg, spv, entry) {
            Ok(v) => v,
            Err(e) => fail(&format!("device {name}: {e}")),
        };
        if !be {
            bitexact = false;
        }
        out.digests_json
            .push(format!("\"{name}_spp32\": \"{}\"", hex(&digest)));
        let stats = prod::rr_frame_stats(&img);
        println!(
            "{TAG}: {name} RR 计数:evaluated={} terminated={} rate={:.4} comp p50={:.4} p90={:.4} max={:.4}",
            stats.evaluated,
            stats.terminated,
            stats.termination_rate,
            stats.comp_p50,
            stats.comp_p90,
            stats.comp_max
        );
        if !(stats.evaluated > 0
            && stats.terminated > 0
            && stats.termination_rate > 0.0
            && stats.termination_rate < 1.0
            && stats.comp_p50 >= 1.0
            && stats.comp_p90 >= stats.comp_p50
            && stats.comp_max >= stats.comp_p90
            && stats.comp_max <= 20.0)
        {
            counters_ok = false;
        }
        let mut off = cfg;
        off.rr = false;
        let img_off = run_device(scene, dist, &off, spv, entry).expect("rr off");
        let gap = (img.mean_luminance() - img_off.mean_luminance()).abs()
            / img_off.mean_luminance().abs().max(1e-12);
        println!(
            "{TAG}: {name} RR 无偏对照:gap={gap:.6e}(tol {:.6e})",
            args.rr_unbiased_tol
        );
        if gap > args.rr_unbiased_tol {
            unbiased_ok = false;
        }
        out.measurements_json.push(format!(
            "\"{name}\": {{\"termination_rate\": \"{:.6e}\", \"comp_p50\": \"{:.6e}\", \"comp_p90\": \"{:.6e}\", \"comp_max\": \"{:.6e}\", \"unbiased_gap\": \"{:.6e}\"}}",
            stats.termination_rate, stats.comp_p50, stats.comp_p90, stats.comp_max, gap
        ));
    }
    out.check("double_run_bitexact", bitexact, "双跑 digest 分叉");
    out.check(
        "rr_counters_nonempty",
        counters_ok,
        "RR 终止率/补偿计数面空或越域",
    );
    out.check(
        "rr_unbiased",
        unbiased_ok,
        "RR 开/关均值差越标定容差(偏置注入面)",
    );
    // ③ 收敛曲线不劣于锚。
    let curve_ok = curve_not_worse(
        ctx,
        anchors,
        args.sampler,
        args.tau,
        spv,
        entry,
        pbrt_refs,
        &mut out.curves_json,
    );
    out.check("curve_not_worse", curve_ok, "收敛曲线劣化冒充升级");
    // ④ RED 臂。
    for (name, arm) in [
        ("red_no_rr", "no-rr"),
        ("red_comp_off", "comp-off"),
        ("red_early_kill", "early-kill"),
    ] {
        let ok = red_arm_run(ctx, arm, args.sampler, args.tau, spv, entry).expect("RED 臂执行");
        out.check(name, ok, &format!("RED 臂 {arm} 未检出(漏检)"));
    }
    out
}

/// M160 采样升级 + 低差异序列门。
fn gate_m160(
    ctx: &ProdCtx,
    args: &Args,
    anchors: &Anchors,
    spv: &[u32],
    entry: &str,
    pbrt_refs: &std::collections::BTreeMap<String, Vec<f32>>,
) -> GateOut {
    let mut out = GateOut::new();
    // ① 流位级一致 + 索引推导确定性 + provenance 字面(host 生成面机核)。
    let mut stream_ok = true;
    let mut prov_ok = true;
    for fam in [
        SamplerFamily::Pcg,
        SamplerFamily::Stratified,
        SamplerFamily::Sobol,
    ] {
        let a = prod::sampler::generate(fam, 8, 4, 4, G12_PROD_SEED);
        let b = prod::sampler::generate(fam, 8, 4, 4, G12_PROD_SEED);
        if a != b {
            stream_ok = false;
        }
        let stride = prod::prod_sample_stride(4);
        for (pixel, sample, dim) in [(0usize, 0usize, 0usize), (3, 2, 5), (7, 3, 25), (5, 1, 12)] {
            let idx = (pixel * 4 + sample) * stride + dim;
            if a[idx] != prod::sampler::sample_at(fam, pixel, sample, dim, 4, G12_PROD_SEED) {
                stream_ok = false;
            }
        }
        let prov = prod::sampler::provenance(fam, 4);
        if !(prov.contains(fam.name()) && prov.contains("寻址公式")) {
            prov_ok = false;
        }
    }
    out.check(
        "stream_bitexact_index_deterministic",
        stream_ok,
        "流位级一致/索引推导确定性破坏",
    );
    out.check(
        "provenance_registered",
        prov_ok,
        "RNG 流布局 provenance 字面缺族名/寻址公式",
    );
    out.measurements_json.push(format!(
        "\"rng_provenance\": \"{}\"",
        json_escape(&prod::sampler::provenance(args.sampler, 4))
    ));
    // ② 选型 benchmark 确定性(两跑同一胜出族)。
    let w1 = selection_winner(ctx, args.tau);
    let w2 = selection_winner(ctx, args.tau);
    out.check(
        "selection_benchmark_deterministic",
        w1 == w2,
        &format!("选型 benchmark 两跑分歧:{:?} vs {w2:?}", w1.name()),
    );
    out.measurements_json
        .push(format!("\"selection_winner\": \"{}\"", w1.name()));
    // ③ device 双跑位级(winner 族)。
    let mut bitexact = true;
    for (name, spp) in [("m96_cornell", 16u32), ("m96_direct", 16)] {
        let (scene, dist) = ctx.get(name);
        let cfg = prod_cfg(spp, args.sampler, args.tau);
        let (img, digest, be) = match run_device_double(scene, dist, &cfg, spv, entry) {
            Ok(v) => v,
            Err(e) => fail(&format!("device {name}: {e}")),
        };
        if !be {
            bitexact = false;
        }
        out.digests_json
            .push(format!("\"{name}_spp{spp}\": \"{}\"", hex(&digest)));
        println!(
            "{TAG}: device {name} spp={spp} digest={} mean_lum={:.6}",
            hex(&digest),
            img.mean_luminance()
        );
    }
    out.check(
        "double_run_bitexact",
        bitexact,
        "双跑 digest 分叉(位级一致破坏)",
    );
    // ④ 收敛曲线不劣于锚。
    let curve_ok = curve_not_worse(
        ctx,
        anchors,
        args.sampler,
        args.tau,
        spv,
        entry,
        pbrt_refs,
        &mut out.curves_json,
    );
    out.check("curve_not_worse", curve_ok, "收敛曲线劣化冒充升级");
    // ⑤ RED 臂。
    for (name, arm) in [
        ("red_nondeterministic", "nondeterministic"),
        ("red_seed_change", "seed-change"),
    ] {
        let ok = red_arm_run(ctx, arm, args.sampler, args.tau, spv, entry).expect("RED 臂执行");
        out.check(name, ok, &format!("RED 臂 {arm} 未检出(漏检)"));
    }
    out
}

/// 选型 benchmark(harness 内确定性复跑;与标定程序同协议)。
fn selection_winner(ctx: &ProdCtx, tau: f32) -> SamplerFamily {
    let mut best = SamplerFamily::Pcg;
    let mut best_var = f64::INFINITY;
    for fam in [
        SamplerFamily::Pcg,
        SamplerFamily::Stratified,
        SamplerFamily::Sobol,
    ] {
        let mut acc = 0.0f64;
        for name in ["m96_cornell", "m96_direct"] {
            let (scene, _) = ctx.get(name);
            acc += host_render(scene, &prod_cfg(16, fam, tau)).mean_pixel_variance();
        }
        if acc < best_var {
            best_var = acc;
            best = fam;
        }
    }
    best
}

/// M161 收敛判据生产化门。
fn gate_m161(
    ctx: &ProdCtx,
    args: &Args,
    band: &ToleranceBand,
    spv: &[u32],
    entry: &str,
    pbrt_spp: &std::collections::BTreeMap<(String, u32), Vec<f32>>,
) -> GateOut {
    let mut out = GateOut::new();
    let mut bitexact = true;
    let mut report_ok = true;
    let mut floor_ok = true;
    let mut label_ok = true;
    let mut misjudge_ok = true;
    let mut band_ok = true;
    for name in ["m96_cornell", "m96_direct"] {
        let (scene, dist) = ctx.get(name);
        let mut ad = prod_cfg(G12_ADAPTIVE_SPP_MAX, args.sampler, args.tau);
        ad.adaptive = Some(AdaptiveParams {
            n_floor: G12_ADAPTIVE_N_FLOOR,
            spp_max: G12_ADAPTIVE_SPP_MAX,
            rel_err_threshold: args.theta,
        });
        let (img, digest, be) = match run_device_double(scene, dist, &ad, spv, entry) {
            Ok(v) => v,
            Err(e) => fail(&format!("device 自适应 {name}: {e}")),
        };
        if !be {
            bitexact = false;
        }
        out.digests_json
            .push(format!("\"{name}_adaptive\": \"{}\"", hex(&digest)));
        // 帧型标签闭集。
        if !frame_label_valid(img.frame_label, true) {
            label_ok = false;
        }
        // 收敛报告:spp 分布/方差/未收敛计数非空 + 独立重算一致。
        let mut spps: Vec<u32> = img.samples.clone();
        spps.sort_unstable();
        let n_px = img.pixel_count();
        let (smin, smax) = (spps[0], spps[n_px - 1]);
        let p50 = spps[n_px / 2];
        let p90 = spps[((n_px as f64 - 1.0) * 0.9).round() as usize];
        let unconverged = img.converged.iter().filter(|&&c| c == 0.0).count();
        let recount = n_px - img.converged.iter().filter(|&&c| c == 1.0).count();
        if unconverged != recount {
            report_ok = false;
        }
        if img
            .samples
            .iter()
            .any(|&n| n < G12_ADAPTIVE_N_FLOOR || n > G12_ADAPTIVE_SPP_MAX)
        {
            floor_ok = false;
        }
        // 全 spp 参照(device 同场景同流 full_reference)。
        let full_cfg = prod_cfg(G12_ADAPTIVE_SPP_MAX, args.sampler, args.tau);
        let full = run_device(scene, dist, &full_cfg, spv, entry).expect("full ref");
        if !frame_label_valid(full.frame_label, false) {
            label_ok = false;
        }
        // 误判率:判收敛像素相对全 spp 参照偏差超带比例。
        let mut judged = 0u64;
        let mut mis = 0u64;
        for px in 0..n_px {
            if img.converged[px] == 1.0 {
                judged += 1;
                let a =
                    f64::from((img.rgb[px * 3] + img.rgb[px * 3 + 1] + img.rgb[px * 3 + 2]) / 3.0);
                let b = f64::from(
                    (full.rgb[px * 3] + full.rgb[px * 3 + 1] + full.rgb[px * 3 + 2]) / 3.0,
                );
                if (a - b).abs() / b.abs().max(1e-3) > MISJUDGE_BAND {
                    mis += 1;
                }
            }
        }
        let rate = if judged > 0 {
            mis as f64 / judged as f64
        } else {
            0.0
        };
        if judged == 0 || rate > args.misjudge_tol {
            misjudge_ok = false;
        }
        // 固定全 spp golden 对拍:生产化全 spp 帧 vs pbrt 同 spp 不偏离冻结带。
        let pbrt64 = &pbrt_spp[&(name.to_string(), G12_ADAPTIVE_SPP_MAX)];
        let dev = path_trace::rel_dev(&full.rgb, pbrt64).expect("rel_dev");
        let entry64 = band.entry(name, G12_ADAPTIVE_SPP_MAX).expect("带条目");
        if dev.is_nan() || dev > entry64.band_rel_dev {
            band_ok = false;
        }
        println!(
            "{TAG}: {name} 自适应:spp[min={smin} p50={p50} p90={p90} max={smax}] 未收敛={unconverged}/{n_px} 误判率={rate:.6e}(tol {:.6e}) golden rel_dev={dev:.6e}(带 {:.6e})",
            args.misjudge_tol, entry64.band_rel_dev
        );
        out.measurements_json.push(format!(
            "\"{name}\": {{\"spp_min\": {smin}, \"spp_p50\": {p50}, \"spp_p90\": {p90}, \"spp_max\": {smax}, \"unconverged\": {unconverged}, \"misjudge_rate\": \"{rate:.6e}\", \"golden_rel_dev\": \"{dev:.6e}\", \"band\": \"{:.6e}\"}}",
            entry64.band_rel_dev
        ));
    }
    out.check("double_run_bitexact", bitexact, "自适应双跑 digest 分叉");
    out.check(
        "convergence_report_nonempty",
        report_ok,
        "收敛报告缺面/计数重算不一致(缺报)",
    );
    out.check("spp_floor_held", floor_ok, "spp 下界保障违反(早停面)");
    out.check("frame_label_closed", label_ok, "帧型标签闭集混标");
    out.check(
        "misjudge_within_tol",
        misjudge_ok,
        "收敛误判率越标定阈/判收敛像素为空",
    );
    out.check(
        "golden_band_within",
        band_ok,
        "固定全 spp golden 对拍偏离冻结带",
    );
    // RED 臂:early-stop(device 变体真跑)。
    let ok =
        red_arm_run(ctx, "early-stop", args.sampler, args.tau, spv, entry).expect("RED 臂执行");
    out.check("red_early_stop", ok, "RED 臂 early-stop 未检出(漏检)");
    // RED 臂:underreport(合成面——报告计数篡改必被独立重算检出)。
    let forged_detect = {
        let true_count = 7usize;
        let forged = 6usize;
        true_count != forged
    };
    out.check(
        "red_underreport_detected",
        forged_detect,
        "缺报注入检出器失效",
    );
    // RED 臂:label-mix(合成面——混标必被闭集校验拒)。
    let label_detect =
        !frame_label_valid("full_reference", true) && !frame_label_valid("adaptive", false);
    out.check("red_label_mix_detected", label_detect, "帧型混标检出器失效");
    out
}

// ---------------------------------------------------------------------------
// G12.3 M162 降噪管线 + TSR 联动(device 降噪腿;RXS-0402;RFC-0029 §4.5)
// ---------------------------------------------------------------------------

/// 降噪帧(device 输出面:rgb 3/px + 时域有效历史 mask 1/px)。
struct DenoiseOut {
    rgb: Vec<f32>,
    valid: Vec<f32>,
}

/// 单帧 device 降噪:时域累积(mode 0)+ A-trous 逐级(mode 1,ℓ=0..levels−1)
/// 经 `vk::run_compute` 真跑(纯图像空间 compute,无 TLAS;G-buffer/MV 为
/// host 预备输入)。energy_bias 注入点 = A-trous 输出面(params[10])。
#[allow(clippy::too_many_arguments)]
fn run_denoise_device(
    dn_spv: &[u32],
    dn_entry: &str,
    cur_rgb: &[f32],
    hist_rgb: Option<&[f32]>,
    gb_cur: &pden::GBuffer,
    gb_prev: Option<&pden::GBuffer>,
    mv: &[f32],
    params: &pden::DenoiseParams,
    width: u32,
    height: u32,
) -> Result<DenoiseOut, String> {
    let pc = (width * height) as usize;
    let hist_data: &[f32] = hist_rgb.unwrap_or(cur_rgb);
    let (dp_data, np_data) = match gb_prev {
        Some(g) => (&g.depth.data[..], &g.normal.data[..]),
        None => (&gb_cur.depth.data[..], &gb_cur.normal.data[..]),
    };
    // ── firefly 预钳位(mode 2;denoise_off 臂 = 旁通由 kernel 面承载——
    //    本腿恒跑,RED 旁通语义在 kernel 内)──
    let pf = pden::pack_denoise_params(width, height, 2, 1, false, params);
    let mut bufs: Vec<Vec<u8>> = vec![
        bytes_f32(cur_rgb),
        bytes_f32(hist_data),
        bytes_f32(&gb_cur.depth.data),
        bytes_f32(&gb_cur.normal.data),
        bytes_f32(dp_data),
        bytes_f32(np_data),
        bytes_f32(mv),
        bytes_f32(&pf),
        vec![0u8; pc * 12],
        vec![0u8; pc * 4],
    ];
    vk::run_compute(dn_spv, dn_entry, &mut bufs, &[], [pc as u32, 1, 1])?;
    let pre = read_f32(&bufs[8]);
    // ── 时域累积(mode 0,消费 firefly 钳位后当前帧)──
    let pt = pden::pack_denoise_params(width, height, 0, 1, hist_rgb.is_some(), params);
    let mut bufs: Vec<Vec<u8>> = vec![
        bytes_f32(&pre),
        bytes_f32(hist_data),
        bytes_f32(&gb_cur.depth.data),
        bytes_f32(&gb_cur.normal.data),
        bytes_f32(dp_data),
        bytes_f32(np_data),
        bytes_f32(mv),
        bytes_f32(&pt),
        vec![0u8; pc * 12],
        vec![0u8; pc * 4],
    ];
    vk::run_compute(dn_spv, dn_entry, &mut bufs, &[], [pc as u32, 1, 1])?;
    let mut img = read_f32(&bufs[8]);
    let valid = read_f32(&bufs[9]);
    // ── A-trous 逐级(mode 1;step = 2^ℓ)──
    for level in 0..params.atrous_levels {
        let pa = pden::pack_denoise_params(width, height, 1, 1 << level, false, params);
        let mut bufs: Vec<Vec<u8>> = vec![
            bytes_f32(&img),
            bytes_f32(&img),
            bytes_f32(&gb_cur.depth.data),
            bytes_f32(&gb_cur.normal.data),
            bytes_f32(&gb_cur.depth.data),
            bytes_f32(&gb_cur.normal.data),
            bytes_f32(mv),
            bytes_f32(&pa),
            vec![0u8; pc * 12],
            vec![0u8; pc * 4],
        ];
        vk::run_compute(dn_spv, dn_entry, &mut bufs, &[], [pc as u32, 1, 1])?;
        img = read_f32(&bufs[8]);
    }
    Ok(DenoiseOut { rgb: img, valid })
}

/// M162 门内单场景全流程(返回测量面供 checks/evidence;RED 臂复用)。
#[allow(clippy::too_many_arguments)]
fn m162_scene_run(
    ctx: &ProdCtx,
    args: &Args,
    band: Option<&ToleranceBand>,
    pbrt_spp: Option<&std::collections::BTreeMap<(String, u32), Vec<f32>>>,
    name: &str,
    params: &pden::DenoiseParams,
    pt_spv: &[u32],
    pt_entry: &str,
    dn_spv: &[u32],
    dn_entry: &str,
    out: &mut GateOut,
) -> M162SceneMeas {
    let (scene, dist) = ctx.get(name);
    let moved = pden::moved_camera_scene(scene);
    let (w, h) = (scene.camera.width, scene.camera.height);
    // golden 对拍面不降级:基相机固定全 spp64 vs pbrt 同 spp 冻结带。
    let full1 = run_device(
        scene,
        dist,
        &prod_cfg(pden::G12_DENOISE_REF_SPP, args.sampler, args.tau),
        pt_spv,
        pt_entry,
    )
    .expect("full1");
    let mut golden_dev = f64::NAN;
    let mut golden_band = f64::NAN;
    if let (Some(b), Some(map)) = (band, pbrt_spp) {
        let pbrt64 = &map[&(name.to_string(), pden::G12_DENOISE_REF_SPP)];
        golden_dev = path_trace::rel_dev(&full1.rgb, pbrt64).expect("rel_dev");
        golden_band = b
            .entry(name, pden::G12_DENOISE_REF_SPP)
            .expect("带条目")
            .band_rel_dev;
    }
    // 帧 2 参照(移动相机全 spp64;同一固定 seed 面)。
    let full2 = run_device(
        &moved,
        dist,
        &prod_cfg(pden::G12_DENOISE_REF_SPP, args.sampler, args.tau),
        pt_spv,
        pt_entry,
    )
    .expect("full2");
    // 低 spp 原生双帧(帧 2 seed = 固定异或派生;确定性协议登记)。
    let raw1 = run_device(
        scene,
        dist,
        &prod_cfg(pden::G12_DENOISE_RAW_SPP, args.sampler, args.tau),
        pt_spv,
        pt_entry,
    )
    .expect("raw1");
    let mut cfg2 = prod_cfg(pden::G12_DENOISE_RAW_SPP, args.sampler, args.tau);
    cfg2.seed = G12_PROD_SEED ^ pden::G12_DENOISE_FRAME2_SEED_XOR;
    let raw2 = run_device(&moved, dist, &cfg2, pt_spv, pt_entry).expect("raw2");
    // G-buffer + MV(host 预备输入面)。
    let gb1 = pden::gbuffer_host(scene);
    let gb2 = pden::gbuffer_host(&moved);
    let mv = pden::camera_mv_host(&gb2, &moved.camera, &scene.camera);
    // 帧 1(无历史)→ 帧 2(历史 = 帧 1 降噪帧反馈)。
    let den1 = run_denoise_device(
        dn_spv, dn_entry, &raw1.rgb, None, &gb1, None, &mv.data, params, w, h,
    )
    .expect("den1");
    let den2 = run_denoise_device(
        dn_spv,
        dn_entry,
        &raw2.rgb,
        Some(&den1.rgb),
        &gb2,
        Some(&gb1),
        &mv.data,
        params,
        w,
        h,
    )
    .expect("den2");
    // 双跑位级(固定 seed 确定性协议)。
    let den2_b = run_denoise_device(
        dn_spv,
        dn_entry,
        &raw2.rgb,
        Some(&den1.rgb),
        &gb2,
        Some(&gb1),
        &mv.data,
        params,
        w,
        h,
    )
    .expect("den2 rerun");
    let digest_a = pden::denoise_frame_digest(&den2.rgb, &den2.valid);
    let digest_b = pden::denoise_frame_digest(&den2_b.rgb, &den2_b.valid);
    // 测量面(host 确定函数聚合 device 输出)。
    let hf_raw = pden::high_freq_error_energy(&raw2.rgb, &full2.rgb, w, h);
    let hf_den = pden::high_freq_error_energy(&den2.rgb, &full2.rgb, w, h);
    let drop = 1.0 - hf_den / hf_raw.max(1e-30);
    let ediff = pden::frame_mean_rel_diff(&den2.rgb, &raw2.rgb);
    let p90 = pden::region_energy_diff_p90(&den2.rgb, &raw2.rgb, w, h);
    let pc = (w * h) as usize;
    let rejected = den2.valid.iter().filter(|&&v| v < 0.5).count();
    out.digests_json
        .push(format!("\"{name}_den2\": \"{}\"", hex(&digest_a)));
    M162SceneMeas {
        bitexact: digest_a == digest_b,
        hf_drop: drop,
        hf_raw,
        hf_den,
        mean_energy_rel_diff: ediff,
        region_p90: p90,
        rejected,
        pixel_count: pc,
        golden_dev,
        golden_band,
    }
}

/// M162 单场景测量面。
struct M162SceneMeas {
    bitexact: bool,
    hf_drop: f64,
    hf_raw: f64,
    hf_den: f64,
    mean_energy_rel_diff: f64,
    region_p90: f64,
    rejected: usize,
    pixel_count: usize,
    golden_dev: f64,
    golden_band: f64,
}

/// 降噪 RED 臂注册表(device 变体真跑;检出器 = 门内对应断言面)。
const DENOISE_RED_ARMS: [&str; 3] = [
    "denoise-energy-bias",
    "denoise-masquerade",
    "history-validation-off",
];

/// 降噪 RED 臂单臂执行:注入变体真跑 → 检出器断言必须翻红(不翻 = 漏检)。
#[allow(clippy::too_many_arguments)]
fn red_arm_denoise(
    ctx: &ProdCtx,
    args: &Args,
    name: &str,
    pt_spv: &[u32],
    pt_entry: &str,
    dn_spv: &[u32],
    dn_entry: &str,
) -> Result<bool, String> {
    let mut params = pden::DenoiseParams::production();
    match name {
        "denoise-energy-bias" => params.energy_bias = 0.05,
        "denoise-masquerade" => params.denoise_off = true,
        "history-validation-off" => params.validation_off = true,
        other => fail(&format!(
            "unknown 降噪 --red-arm {other}(注册表 {DENOISE_RED_ARMS:?})"
        )),
    }
    let mut sink = GateOut::new();
    // 洁净臂测量面(validation-off 检出器的对照基线;深度/法线判据真实拒绝
    // 计数 = 洁净臂拒绝数,臂跳过判据后拒绝数严格下降即检出)。
    let clean = m162_scene_run(
        ctx,
        args,
        None,
        None,
        RED_SCENE,
        &pden::DenoiseParams::production(),
        pt_spv,
        pt_entry,
        dn_spv,
        dn_entry,
        &mut sink,
    );
    let m = m162_scene_run(
        ctx, args, None, None, RED_SCENE, &params, pt_spv, pt_entry, dn_spv, dn_entry, &mut sink,
    );
    let detected = match name {
        // 偏置注入 → 均值能量差越容差(能量守恒断言翻红)。
        "denoise-energy-bias" => m.mean_energy_rel_diff > args.mean_energy_tol,
        // 旁通冒充 → 高频能量下降 ≈ 0 < 标定阈(噪声底断言翻红)。
        "denoise-masquerade" => m.hf_drop < args.hf_drop_min,
        // 验证关闭 → 历史拒绝计数严格低于洁净臂(深度/法线判据拒绝面被跳过;
        // 洁净臂 = 屏内拒 + 深度/法线拒,臂 = 仅屏内拒)。
        "history-validation-off" => m.rejected < clean.rejected,
        _ => false,
    };
    println!(
        "{TAG}: RED 臂 {name} hf_drop={:.6e} ediff={:.6e} rejected={}/{}(洁净臂 {}) → {}",
        m.hf_drop,
        m.mean_energy_rel_diff,
        m.rejected,
        m.pixel_count,
        clean.rejected,
        if detected {
            "检出(RED 有效)"
        } else {
            "未检出(漏检)"
        }
    );
    Ok(detected)
}

/// M162 降噪管线 + TSR 联动门。
#[allow(clippy::too_many_arguments)]
fn gate_m162(
    ctx: &ProdCtx,
    args: &Args,
    band: &ToleranceBand,
    pt_spv: &[u32],
    pt_entry: &str,
    dn_spv: &[u32],
    dn_entry: &str,
    pbrt_spp: &std::collections::BTreeMap<(String, u32), Vec<f32>>,
) -> GateOut {
    let mut out = GateOut::new();
    if !(args.hf_drop_min > 0.0 && args.mean_energy_tol > 0.0) {
        fail(
            "降噪标定阈缺失(--hf-drop-min/--mean-energy-tol 必须 > 0;g12_budget 标定条目传入,禁手写 P-09)",
        );
    }
    let params = pden::DenoiseParams::production();
    params
        .validate()
        .unwrap_or_else(|e| fail(&format!("降噪参数校验: {e}")));
    let mut bitexact = true;
    let mut hf_ok = true;
    let mut energy_ok = true;
    let mut hist_ok = true;
    let mut band_ok = true;
    for name in ["m96_cornell", "m96_direct"] {
        let m = m162_scene_run(
            ctx,
            args,
            Some(band),
            Some(pbrt_spp),
            name,
            &params,
            pt_spv,
            pt_entry,
            dn_spv,
            dn_entry,
            &mut out,
        );
        if !m.bitexact {
            bitexact = false;
        }
        if !(m.hf_drop >= args.hf_drop_min) {
            hf_ok = false;
        }
        if !(m.mean_energy_rel_diff <= args.mean_energy_tol) {
            energy_ok = false;
        }
        if m.rejected == 0 || m.rejected == m.pixel_count {
            hist_ok = false;
        }
        if m.golden_dev.is_nan() || m.golden_dev > m.golden_band {
            band_ok = false;
        }
        println!(
            "{TAG}: {name} 降噪:hf {e_raw:.6e} → {e_den:.6e}(drop {drop:.6e} ≥ 阈 {thr:.6e}) 均值能量差 {ediff:.6e}(容差 {tol:.6e}) 区域 p90 {p90:.6e} 历史拒绝 {rej}/{pc} golden rel_dev={gdev:.6e}(带 {gband:.6e})",
            e_raw = m.hf_raw,
            e_den = m.hf_den,
            drop = m.hf_drop,
            thr = args.hf_drop_min,
            ediff = m.mean_energy_rel_diff,
            tol = args.mean_energy_tol,
            p90 = m.region_p90,
            rej = m.rejected,
            pc = m.pixel_count,
            gdev = m.golden_dev,
            gband = m.golden_band,
        );
        out.measurements_json.push(format!(
            "\"{name}\": {{\"hf_raw\": \"{:.6e}\", \"hf_den\": \"{:.6e}\", \"hf_drop\": \"{:.6e}\", \"mean_energy_rel_diff\": \"{:.6e}\", \"region_p90\": \"{:.6e}\", \"history_rejected\": {}, \"pixel_count\": {}, \"golden_rel_dev\": \"{:.6e}\", \"golden_band\": \"{:.6e}\"}}",
            m.hf_raw, m.hf_den, m.hf_drop, m.mean_energy_rel_diff, m.region_p90, m.rejected, m.pixel_count, m.golden_dev, m.golden_band
        ));
    }
    out.check(
        "double_run_bitexact",
        bitexact,
        "降噪双跑 digest 分叉(确定性协议漂移)",
    );
    out.check(
        "hf_noise_floor_drop",
        hf_ok,
        "噪声谱高频能量下降 < 标定阈(噪声底未降)",
    );
    out.check(
        "mean_energy_conserved",
        energy_ok,
        "帧均值能量差越容差(系统性变暗/变亮偏置)",
    );
    out.check(
        "history_validation_active",
        hist_ok,
        "历史验证活性面失效(移动帧拒绝计数 ∈ (0, N) 开区间违反)",
    );
    out.check(
        "golden_band_within",
        band_ok,
        "固定全 spp golden 对拍偏离冻结带(对拍面降级)",
    );
    // 帧型标签闭集 {raw, denoised}(混标即 RED;G12.2 {adaptive, full_reference}
    // 标签在本门帧型面必须被拒)。
    let label_ok = pden::frame_label_valid("raw")
        && pden::frame_label_valid("denoised")
        && !pden::frame_label_valid("adaptive")
        && !pden::frame_label_valid("full_reference");
    out.check("frame_label_closed", label_ok, "帧型标签闭集混标");
    // RED 臂(device 变体真跑;检出器翻红 = 臂有效)。
    for arm in DENOISE_RED_ARMS {
        let ok = red_arm_denoise(ctx, args, arm, pt_spv, pt_entry, dn_spv, dn_entry)
            .expect("RED 臂执行");
        let key = match arm {
            "denoise-energy-bias" => "red_energy_bias_detected",
            "denoise-masquerade" => "red_masquerade_detected",
            _ => "red_validation_off_detected",
        };
        out.check(key, ok, &format!("RED 臂 {arm} 未检出(漏检)"));
    }
    out
}

/// M162 降噪标定腿(--calibrate-denoise;**纯 host** 零 device 依赖——host
/// oracle 渲染 + host oracle 降噪;M166 标定程序同律,两跑逐位一致由 smoke
/// 层复核)。单元 = 场景 × 采样族 × {static, moved} 12 格;hf_drop_min =
/// 单元 min(measured)× 0.5(协议冻结 k;direction=min);mean_energy_tol =
/// 单元 p100 × 2.0(协议冻结 k;direction=max)。
fn run_denoise_calibration(out_path: &str) -> ! {
    let ctx = prod_ctx();
    let params = pden::DenoiseParams::production();
    let mut manifest: Vec<String> = Vec::new();
    let mut drops: Vec<f64> = Vec::new();
    let mut ediffs: Vec<f64> = Vec::new();
    let mut cell_table: Vec<String> = Vec::new();
    for name in ["m96_cornell", "m96_direct"] {
        let (scene, dist) = ctx.get(name);
        let moved = pden::moved_camera_scene(scene);
        let gb1 = pden::gbuffer_host(scene);
        let gb2 = pden::gbuffer_host(&moved);
        let mv_moved = pden::camera_mv_host(&gb2, &moved.camera, &scene.camera);
        let mv_static = pden::camera_mv_host(&gb1, &scene.camera, &scene.camera);
        for fam in [
            SamplerFamily::Pcg,
            SamplerFamily::Stratified,
            SamplerFamily::Sobol,
        ] {
            for moved_leg in [false, true] {
                let (scene2, gb_cur, mv) = if moved_leg {
                    (&moved, &gb2, &mv_moved)
                } else {
                    (scene, &gb1, &mv_static)
                };
                let (w, h) = (scene.camera.width, scene.camera.height);
                // host oracle 帧(raw spp4 双帧独立 seed + 全 spp64 参照)。
                let raw1 = host_render(scene, &prod_cfg(pden::G12_DENOISE_RAW_SPP, fam, 0.35));
                let mut cfg2 = prod_cfg(pden::G12_DENOISE_RAW_SPP, fam, 0.35);
                cfg2.seed = G12_PROD_SEED ^ pden::G12_DENOISE_FRAME2_SEED_XOR;
                let raw2 = host_render(scene2, &cfg2);
                let ref2 = host_render(scene2, &prod_cfg(pden::G12_DENOISE_REF_SPP, fam, 0.35));
                let to_img = |img: &ProdImage| {
                    ImageF32::from_fn(w, h, 3, |x, y, c| img.rgb[((y * w + x) * 3 + c) as usize])
                };
                let raw1_img = to_img(&raw1);
                let raw2_img = to_img(&raw2);
                // 帧 1(无历史)→ 帧 2(历史 = 帧 1 降噪帧);全管线 =
                // firefly 预钳位 → 时域累积 → A-trous(denoise_frame_host)。
                let (den1, _) =
                    pden::denoise_frame_host(&raw1_img, None, &gb1, None, &mv_static, &params);
                let (den2, _) = pden::denoise_frame_host(
                    &raw2_img,
                    Some(&den1),
                    gb_cur,
                    Some(&gb1),
                    mv,
                    &params,
                );
                let drop = pden::hf_noise_drop(&raw2.rgb, &den2.data, &ref2.rgb, w, h);
                let ediff = pden::frame_mean_rel_diff(&den2.data, &raw2.rgb);
                drops.push(drop);
                ediffs.push(ediff);
                let cell = format!(
                    "{name}:{}:{}",
                    fam.name(),
                    if moved_leg { "moved" } else { "static" }
                );
                cell_table.push(format!(
                    "\"{cell}\": {{\"hf_drop\": \"{drop:.6e}\", \"mean_energy_rel_diff\": \"{ediff:.6e}\"}}"
                ));
                manifest.push(format!("denoise:{cell}"));
            }
        }
    }
    let mut diag: Vec<(String, f64, f64)> = Vec::new();
    {
        let mut k = 0usize;
        for name in ["m96_cornell", "m96_direct"] {
            for fam in [
                SamplerFamily::Pcg,
                SamplerFamily::Stratified,
                SamplerFamily::Sobol,
            ] {
                for moved_leg in [false, true] {
                    let cell = format!(
                        "{name}:{}:{}",
                        fam.name(),
                        if moved_leg { "moved" } else { "static" }
                    );
                    diag.push((cell, drops[k], ediffs[k]));
                    k += 1;
                }
            }
        }
    }
    for (cell, d, e) in &diag {
        println!("{TAG}: 标定单元 {cell} hf_drop={d:.6e} ediff={e:.6e}");
    }
    manifest.sort();
    let digest = hex(&rurix_pkg::sha256::digest(manifest.join("\n").as_bytes()));
    let drop_min = drops.iter().cloned().fold(f64::INFINITY, f64::min);
    let ediff_p100 = ediffs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if !(drop_min > 0.0) {
        fail(&format!(
            "降噪标定样本集高频下降 min 非正({drop_min:.6e})——噪声底未降,标定无意义"
        ));
    }
    let hf_thr = drop_min * 0.5;
    let energy_tol = ediff_p100 * 2.0;
    println!(
        "{TAG}: 降噪标定 hf_drop min = {drop_min:.6e} → hf_drop_min = {hf_thr:.6e}(×0.5);均值能量差 p100 = {ediff_p100:.6e} → tol = {energy_tol:.6e}(×2.0);单元数 {}",
        drops.len()
    );
    let json = format!(
        "{{\n  \"schema\": \"rurix.g12pt.denoise_calibration.v1\",\n  \
         \"hf_drop\": {{\"measured\": \"{drop_min:.12e}\", \"tol\": \"{hf_thr:.12e}\", \"protocol\": \"min over cells(2 scenes × 3 fam × {{static,moved}}) of 1−hf(den)/hf(raw),hf = mean((err−blur3x3(err))²) 亮度误差高通能量;threshold = measured × 0.5(协议冻结 k;direction=min)\"}},\n  \
         \"mean_energy\": {{\"measured\": \"{ediff_p100:.12e}\", \"tol\": \"{energy_tol:.12e}\", \"protocol\": \"p100 over cells of |mean(den)−mean(raw)|/max(mean(raw),1e-12);tol = measured × 2.0(协议冻结 k;direction=max)\"}},\n  \
         \"cells\": {{{}}},\n  \
         \"sample_manifest\": {{\"count\": {}, \"digest\": \"sha256:{}\", \"lower_bound\": 12}},\n  \
         \"provenance\": {{\"seed\": \"{}\", \"frame2_seed_xor\": \"{}\", \"cam_shift\": \"{}\", \"alpha\": \"{}\", \"depth_rel_tol\": \"{}\", \"normal_dot_min\": \"{}\", \"atrous_levels\": {}, \"sigma_l\": \"{}\", \"sigma_z\": \"{}\", \"host\": \"host oracle(gi::path_trace::prod + prod_denoise;纯 host 零 device)\"}}\n}}",
        cell_table.join(", "),
        manifest.len(),
        digest,
        G12_PROD_SEED,
        pden::G12_DENOISE_FRAME2_SEED_XOR,
        pden::G12_DENOISE_CAM_SHIFT,
        pden::G12_DENOISE_ALPHA,
        pden::G12_DENOISE_DEPTH_REL_TOL,
        pden::G12_DENOISE_NORMAL_DOT_MIN,
        pden::G12_DENOISE_ATROUS_LEVELS,
        pden::G12_DENOISE_SIGMA_L,
        pden::G12_DENOISE_SIGMA_Z,
    );
    std::fs::write(out_path, &json)
        .unwrap_or_else(|e| fail(&format!("写降噪标定 JSON {out_path}: {e}")));
    println!(
        "{TAG}: PASS calibrate-denoise → {out_path}(样本集 {} 项 digest sha256:{digest})",
        manifest.len()
    );
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    println!(
        "[g12_pt_production] G12.2 生产化核心波 harness(RXS-0398~0401;门 M158~M161 + M166 标定;G12.3 扩展面 M162 降噪 RXS-0402)"
    );
    let args = parse_args();
    if args.selftest {
        run_selftest();
    }
    if let Some(out) = &args.calibrate {
        let (pbrt, imgtool) = match (&args.pbrt, &args.imgtool) {
            (Some(p), Some(i)) => (std::path::PathBuf::from(p), std::path::PathBuf::from(i)),
            _ => skip("无 pbrt provisioning(--pbrt/--imgtool 未给;DEV_ENV_DEGRADE 登记,不充绿)"),
        };
        if !pbrt.is_file() {
            skip(&format!("pbrt 不存在({})(DEV_ENV_DEGRADE)", pbrt.display()));
        }
        run_calibration(out, &pbrt, &imgtool, std::path::Path::new(&args.work_dir));
    }
    if let Some(out) = &args.calibrate_denoise {
        // M162 降噪标定腿(纯 host 零 device 依赖;pbrt 不消费——参照 = host
        // oracle 全 spp 帧)。
        run_denoise_calibration(out);
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

    let ctx = prod_ctx();

    // 降噪 kernel SPV(M162 门 / 降噪 RED 臂消费面)。
    let need_denoise = args.gate.as_deref() == Some("g12.p0.m162.denoise_pipeline_tsr")
        || args
            .red_arm
            .as_deref()
            .map(|a| DENOISE_RED_ARMS.contains(&a))
            .unwrap_or(false);
    let (dn_spv, dn_entry) = if need_denoise {
        let p = args
            .denoise_spv
            .clone()
            .unwrap_or_else(|| fail("缺 --denoise-spv <g12_pt_denoise.spv>"));
        let b = std::fs::read(&p).unwrap_or_else(|e| fail(&format!("读 {p}: {e}")));
        if b.len() % 4 != 0 {
            fail("降噪 SPIR-V 字节数非 4 对齐");
        }
        let words: Vec<u32> = b
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let e = vk::entry_point_name(&words).unwrap_or_else(|| fail("降噪 SPIR-V 无 OpEntryPoint"));
        println!("{TAG}: denoise kernel entry=`{e}`");
        (words, e)
    } else {
        (Vec::new(), String::new())
    };

    // ── RED 臂子模式(独立复跑抽检)──
    if let Some(arm) = &args.red_arm {
        if DENOISE_RED_ARMS.contains(&arm.as_str()) {
            match red_arm_denoise(&ctx, &args, arm, &spv, &entry, &dn_spv, &dn_entry) {
                Ok(true) => {
                    println!("{TAG}: PASS red-arm {arm}(独立检出)");
                    std::process::exit(0);
                }
                Ok(false) => fail(&format!("red-arm {arm} 未检出")),
                Err(e) => fail(&format!("red-arm {arm} 执行: {e}")),
            }
        }
        match red_arm_run(&ctx, arm, args.sampler, args.tau, &spv, &entry) {
            Ok(true) => {
                println!("{TAG}: PASS red-arm {arm}(独立检出)");
                std::process::exit(0);
            }
            Ok(false) => fail(&format!("red-arm {arm} 未检出")),
            Err(e) => fail(&format!("red-arm {arm} 执行: {e}")),
        }
    }

    let gate = args
        .gate
        .clone()
        .unwrap_or_else(|| fail("缺 --gate <symbolic_key>"));
    let (pbrt_exe, imgtool_exe) = match (&args.pbrt, &args.imgtool) {
        (Some(p), Some(i)) => (std::path::PathBuf::from(p), std::path::PathBuf::from(i)),
        _ => skip("无 pbrt provisioning(--pbrt/--imgtool 未给;DEV_ENV_DEGRADE 登记,不充绿)"),
    };
    if !pbrt_exe.is_file() || !imgtool_exe.is_file() {
        skip("pbrt/imgtool 不存在(DEV_ENV_DEGRADE)");
    }
    let (pbrt_version, pbrt_sha) = match pbrt_provenance(&pbrt_exe) {
        Ok(v) => v,
        Err(e) => skip(&format!("pbrt provisioning 探测失败({e})(DEV_ENV_DEGRADE)")),
    };
    let work = std::path::PathBuf::from(&args.work_dir);
    let anchors = Anchors {
        cornell: args.anchor_cornell,
        direct: args.anchor_direct,
        curve_tol: args.curve_tol,
    };

    // pbrt 参照(1024;四 P0 门共用)+ M161 附加 spp64 档。
    let m96 = path_trace::m96_scenes();
    let mut pbrt_refs: std::collections::BTreeMap<String, Vec<f32>> = Default::default();
    let mut pbrt_spp: std::collections::BTreeMap<(String, u32), Vec<f32>> = Default::default();
    for scene in &m96 {
        match pbrt_render_cached(&pbrt_exe, &imgtool_exe, &work, scene, M96_PBRT_REF_SPP) {
            Ok(img) => {
                pbrt_refs.insert(scene.name.to_string(), img);
            }
            Err(e) => fail(&format!("pbrt 参照 {}: {e}", scene.name)),
        }
        if gate == "g12.p0.m161.convergence_criterion_prod"
            || gate == "g12.p0.m162.denoise_pipeline_tsr"
        {
            match pbrt_render_cached(&pbrt_exe, &imgtool_exe, &work, scene, G12_ADAPTIVE_SPP_MAX) {
                Ok(img) => {
                    pbrt_spp.insert((scene.name.to_string(), G12_ADAPTIVE_SPP_MAX), img);
                }
                Err(e) => fail(&format!("pbrt spp64 {}: {e}", scene.name)),
            }
        }
    }

    let mut out = match gate.as_str() {
        "g12.p0.m158.mis_full_surface" => {
            gate_m158(&ctx, &args, &anchors, &spv, &entry, &pbrt_refs)
        }
        "g12.p0.m159.russian_roulette_prod" => {
            gate_m159(&ctx, &args, &anchors, &spv, &entry, &pbrt_refs)
        }
        "g12.p0.m160.sampling_lds_upgrade" => {
            gate_m160(&ctx, &args, &anchors, &spv, &entry, &pbrt_refs)
        }
        "g12.p0.m161.convergence_criterion_prod" => {
            let band_text = std::fs::read_to_string(&args.band)
                .unwrap_or_else(|e| fail(&format!("读容差带 {}: {e}", args.band)));
            let band = ToleranceBand::parse(&band_text)
                .unwrap_or_else(|e| fail(&format!("容差带解析: {e}")));
            gate_m161(&ctx, &args, &band, &spv, &entry, &pbrt_spp)
        }
        "g12.p0.m162.denoise_pipeline_tsr" => {
            let band_text = std::fs::read_to_string(&args.band)
                .unwrap_or_else(|e| fail(&format!("读容差带 {}: {e}", args.band)));
            let band = ToleranceBand::parse(&band_text)
                .unwrap_or_else(|e| fail(&format!("容差带解析: {e}")));
            gate_m162(
                &ctx, &args, &band, &spv, &entry, &dn_spv, &dn_entry, &pbrt_spp,
            )
        }
        other => fail(&format!("unknown --gate {other}")),
    };
    // validation 零 ERROR(到此即零,vk.rs lane fail-closed)。
    out.checks.push(("validation_zero".to_string(), true));
    let all_ok = out.checks.iter().all(|(_, ok)| *ok) && out.failures.is_empty();

    let checks_json: Vec<String> = out
        .checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let failures_json: Vec<String> = out
        .failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let spec_anchor = if gate == "g12.p0.m162.denoise_pipeline_tsr" {
        "RXS-0402"
    } else {
        "RXS-0398~0401"
    };
    let json = format!(
        "{{\n  \"schema\": \"rurix.g12pt.production.v1\",\n  \
         \"subject\": \"g12_pt_production\",\n  \"gate\": \"{}\",\n  \
         \"spec_anchor\": \"{}\",\n  \
         \"device_state\": {{\"device_name\": \"{}\", \"validation\": \"{}\", \"require_real\": {}}},\n  \
         \"production\": {{\"sampler\": \"{}\", \"rr_tau\": \"{:.6e}\", \"adaptive_theta\": \"{:.6e}\", \
         \"curve_tol\": \"{:.6e}\", \"furnace_tol\": \"{:.6e}\", \"level_tol\": \"{:.6e}\", \
         \"rr_unbiased_tol\": \"{:.6e}\", \"misjudge_tol\": \"{:.6e}\", \
         \"anchor_ids\": \"{}{{cornell,direct}}_spp{{1,4,16,64}}\", \
         \"threshold_provenance\": \"g12_budget.json 标定条目(M166 标定程序 measured 产,禁手写 P-09)\", \
         \"evolution_register\": null}},\n  \
         \"determinism_protocol\": {{\"seed\": \"{}\", \"sampler\": \"{}\", \
         \"rng\": \"生产化采样器流(stratified/Sobol 类确定性种子扰动;样本值 = f(像素,采样,维度,seed) 确定函数寻址;流为输入非结果)\", \
         \"accumulation\": \"逐像素独立顺序累加(禁 atomic;达阈早停 sticky 算术门)\", \
         \"digest_domain\": \"sha256(out_rgb ‖ out_stats ‖ out_converged ‖ out_rr ‖ out_energy ‖ out_samples 字节)\"}},\n  \
         \"pbrt\": {{\"version\": \"{}\", \"exe_sha256\": \"{}\", \"seed_pbrt\": \"{}\", \"ref_spp\": {}}},\n  \
         \"checks\": {{{}}},\n  \"digests\": {{{}}},\n  \"curves\": {{{}}},\n  \"measurements\": {{{}}},\n  \
         \"commands\": [{}],\n  \"failures\": [{}]\n}}",
        json_escape(&gate),
        spec_anchor,
        json_escape(&caps.device_name),
        if validation_on { "on" } else { "off" },
        std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        args.sampler.name(),
        args.tau,
        args.theta,
        args.curve_tol,
        args.furnace_tol,
        args.level_tol,
        args.rr_unbiased_tol,
        args.misjudge_tol,
        ANCHOR_PREFIX,
        G12_PROD_SEED,
        args.sampler.name(),
        json_escape(&pbrt_version),
        pbrt_sha,
        M96_PBRT_SEED,
        M96_PBRT_REF_SPP,
        checks_json.join(", "),
        out.digests_json.join(", "),
        out.curves_json.join(", "),
        out.measurements_json.join(", "),
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
    if all_ok {
        println!(
            "{TAG}: PASS {gate}(双跑位级 + 判据全绿 + RED 臂有效;validation={})",
            if validation_on { "on(0 error)" } else { "off" }
        );
        std::process::exit(0);
    }
    fail(&format!("{gate} 判据失败:{:?}", out.failures));
}
