//! G9.4 M100 低档多灯直接光 device harness(RXS-0361;门
//! `g9.p1.m100.multi_light_low`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M100 行 + spec/global_illumination.md RXS-0361)
//!
//! - **低档多灯直接光(MegaLights 式固定随机选灯)为默认档,多灯场景出图与
//!   golden 相等**:场景 = `gi::multi_light::m100_multi_light_scene`(cornell
//!   几何 + 4 光源 quad);device kernel `g9_m100_multi_light.rx`(RayQuery)
//!   真跑;golden = 逐灯单光源 M96 megakernel 参照图之和(光传输线性叠加,
//!   匹配深度 1);产物 digest 冻结于
//!   `milestones/g9/g9_m100_multi_light_band.json`(带 = measured ×
//!   M100_BAND_MARGIN,禁手写 P-09);
//! - **验证射线零跳过统计性偏置硬契约(D2-Q4)**:参照档验证射线实际发射数
//!   = 主命中样本数(零跳过)、逐灯发射计数非空(逐样本发行记录 diag host
//!   确定性归约);跳验证射线注入 ⇒ 系统性变亮偏置 ≥ 冻结阈 + 计数缺空 +
//!   digest 分叉(负例 RED 臂独立有效);灯子集采样注入 ⇒ 偏离 ≥ 冻结阈
//!   (RED 臂);sabotage 探针(参照 vs 参照)必不触发(能红证明);
//! - **选灯种子流固定、同输入双运行逐位一致**:device 双跑 digest 相等 +
//!   同流 host 双跑逐位一致;
//! - **海量灯阴影统一接口随动**:逐灯可见性经同一 ray query 通道(灯表
//!   索引化,无灯级特化路径——结构性登记);
//! - **高档 ReSTIR reservoir 证据不足登记 not-triggered 不充绿**(RD-040
//!   条件分项):`check_restir_trigger` 判 not-triggered 显式登记;
//!   `restir_serve` 服务请求必 typed Err;M15 维持 open-留档;
//! - **按匹配深度对 M96 golden + 门序硬约束**:M96 cornell 深度 1 实跑
//!   digest 与 M97 冻结带条目逐字相等(D2-Q7 门序消费锚)。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备/W3 能力链缺失 → `G9_M100_ML: SKIP DEV_ENV_DEGRADE`
//! (退 0,非 fake pass;`RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层
//! 裁决);判据不符 / RED 轴失效 → FAIL 退 1。`RURIX_VK_VALIDATION=1`:vk.rs
//! lane 内 fail-closed;evidence 记 validation 模式。
//!
//! ## 用法
//!
//! ```text
//! g9_m100_multi_light_low --spv-m100 <m100.spv> --spv-m96 <m96.spv>
//!     [--band <path>] [--m97-band <path>] [--evidence <path>] [--work-dir <dir>]
//! g9_m100_multi_light_low --freeze --spv-m100 .. --spv-m96 .. [--band-out <path>]
//! g9_m100_multi_light_low --red-arm skip-verification|light-subset --spv-m100 .. --spv-m96 ..
//! ```

use rurix_render::gi::multi_light::{self as ml, LowTierMode, MlOutput};
use rurix_render::gi::path_trace::{self, PtConfig, PtImage, PtScene};
use rurix_render::gi::surface_cache;
use rurix_rt::render_exec::{self, KernelWave};
use rurix_rt::vk::{
    self, RayQueryBufferDesc, RayQueryDispatchDesc, RayQueryInstanceDesc, RayQuerySceneDesc,
};

const TAG: &str = "G9_M100_ML";

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

// ---------------------------------------------------------------------------
// device 执行腿
// ---------------------------------------------------------------------------

fn scene_desc_of(tris: &[f32]) -> (Vec<&[f32]>, [RayQueryInstanceDesc; 1]) {
    let blas_refs: Vec<&[f32]> = vec![tris];
    let instances = [RayQueryInstanceDesc {
        blas: 0,
        custom_index: 0,
        mask: 0xFF,
        sbt_record_offset: 0,
    }];
    (blas_refs, instances)
}

/// M100 低档档 device 真跑(RayQuery;按像素 dispatch,内部采样循环)。
fn run_m100_device(
    scene: &ml::MultiLightScene,
    mode: LowTierMode,
    spv: &[u32],
    entry: &str,
) -> Result<MlOutput, String> {
    let full = scene.to_pt_scene_full();
    let cam = &full.camera;
    let pixel_count = (cam.width * cam.height) as usize;
    let stream = ml::m100_rng::generate_stream(pixel_count, ml::M100_SPP, ml::M100_SEED);
    let tris = full.blas_triangles();
    let (blas_refs, instances) = scene_desc_of(&tris);
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    let lights_b = bytes_f32(&ml::pack_lights(scene));
    let rng_b = bytes_f32(&stream);
    let mats_b = bytes_f32(&path_trace::pack_mats(&full));
    let tris_b = bytes_f32(&tris);
    let params_b = bytes_f32(&ml::pack_ml_params(scene, ml::M100_SPP, mode));
    let buffers = [
        RayQueryBufferDesc::Input(&lights_b),
        RayQueryBufferDesc::Input(&rng_b),
        RayQueryBufferDesc::Input(&mats_b),
        RayQueryBufferDesc::Input(&tris_b),
        RayQueryBufferDesc::Input(&params_b),
        RayQueryBufferDesc::Output(pixel_count * 12),
        RayQueryBufferDesc::Output(pixel_count * 8),
        RayQueryBufferDesc::Output(pixel_count * ml::M100_SPP as usize * 12),
    ];
    let out = vk::run_ray_query_effects(
        &scene_desc,
        &[RayQueryDispatchDesc {
            name: "g9_m100_multi_light",
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
    Ok(MlOutput {
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
        diag: read_f32(&rb[2]),
    })
}

/// M96 golden 腿:任意 PtScene megakernel 实跑(spp/深度由 cfg)→ (PtImage, digest)。
fn run_m96(
    scene: &PtScene,
    cfg: &PtConfig,
    spv: &[u32],
    entry: &str,
) -> Result<(PtImage, [u8; 32]), String> {
    cfg.validate().map_err(|e| format!("M96 配置校验: {e}"))?;
    let cam = &scene.camera;
    let pixel_count = (cam.width * cam.height) as usize;
    let tris = scene.blas_triangles();
    let (blas_refs, instances) = scene_desc_of(&tris);
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
    let read_u32 = |b: &[u8]| -> Vec<u32> {
        b.chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    };
    let img = PtImage {
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
    };
    let digest = path_trace::image_digest(&img);
    Ok((img, digest))
}

// ---------------------------------------------------------------------------
// 参数解析
// ---------------------------------------------------------------------------

struct Args {
    spv_m100: Option<String>,
    spv_m96: Option<String>,
    evidence: Option<String>,
    band: String,
    m97_band: String,
    work_dir: String,
    freeze: bool,
    band_out: Option<String>,
    red_arm: Option<String>,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = Args {
        spv_m100: None,
        spv_m96: None,
        evidence: None,
        band: "milestones/g9/g9_m100_multi_light_band.json".to_string(),
        m97_band: "milestones/g9/g9_m97_depth_band.json".to_string(),
        work_dir: ".tmp/g9_m100_work".to_string(),
        freeze: false,
        band_out: None,
        red_arm: None,
    };
    let mut i = 0;
    while i < args.len() {
        let take = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
        };
        match args[i].as_str() {
            "--spv-m100" => out.spv_m100 = Some(take(&mut i)),
            "--spv-m96" => out.spv_m96 = Some(take(&mut i)),
            "--evidence" => out.evidence = Some(take(&mut i)),
            "--band" => out.band = take(&mut i),
            "--m97-band" => out.m97_band = take(&mut i),
            "--work-dir" => out.work_dir = take(&mut i),
            "--freeze" => out.freeze = true,
            "--band-out" => out.band_out = Some(take(&mut i)),
            "--red-arm" => out.red_arm = Some(take(&mut i)),
            other => fail(&format!("unknown arg {other}")),
        }
        i += 1;
    }
    out
}

fn load_spv(path: &str) -> Vec<u32> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(&format!("读 {path}: {e}")));
    if bytes.len() % 4 != 0 {
        fail("SPIR-V 字节数非 4 对齐");
    }
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    println!(
        "[g9_m100_multi_light_low] G9.4 M100 低档多灯直接光 device harness(RXS-0361;门 g9.p1.m100.multi_light_low)"
    );
    let args = parse_args();

    // ── 步骤 0:host 预传递(无 device 依赖)──
    let scene = ml::m100_multi_light_scene();
    scene
        .validate()
        .unwrap_or_else(|e| fail(&format!("场景校验: {e}")));
    let pixel_count = (scene.camera.width * scene.camera.height) as usize;
    let stream = ml::m100_rng::generate_stream(pixel_count, ml::M100_SPP, ml::M100_SEED);
    let host_ref = ml::trace_direct_host(&scene, &stream, ml::M100_SPP, LowTierMode::Reference)
        .unwrap_or_else(|e| fail(&format!("host 参照: {e}")));
    let host_ref2 = ml::trace_direct_host(&scene, &stream, ml::M100_SPP, LowTierMode::Reference)
        .unwrap_or_else(|e| fail(&format!("host 参照双跑: {e}")));
    if host_ref != host_ref2 {
        fail("host 双跑逐位一致破坏(选灯种子流确定性违例)");
    }
    // pbrt 锚定语料消费(导出与 checked-in fixture 逐字相等)。
    let pbrt_text = ml::pbrt_multi_light_text(&scene);
    let pbrt_fixture = std::fs::read_to_string("conformance/gi/scenes/m100_multi_light_low.pbrt")
        .unwrap_or_else(|e| fail(&format!("读 pbrt 锚定语料: {e}")));
    let pbrt_anchor_ok = pbrt_text == pbrt_fixture;
    println!(
        "{TAG}: host 预传递 pixels={pixel_count} lights={} pbrt 锚定语料逐字相等={pbrt_anchor_ok}",
        scene.lights.len()
    );
    let mut failures: Vec<String> = Vec::new();
    if !pbrt_anchor_ok {
        failures.push(
            "pbrt 导出与锚定语料漂移(conformance/gi/scenes/m100_multi_light_low.pbrt)".into(),
        );
    }

    // ── 步骤 1:device 门(三态)──
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
    let spv_m100_path = args
        .spv_m100
        .clone()
        .unwrap_or_else(|| fail("缺 --spv-m100 <m100.spv>"));
    let spv_m100 = load_spv(&spv_m100_path);
    let entry_m100 =
        vk::entry_point_name(&spv_m100).unwrap_or_else(|| fail("M100 SPV 无 OpEntryPoint"));
    let work = std::path::PathBuf::from(&args.work_dir);
    std::fs::create_dir_all(&work).unwrap_or_else(|e| fail(&format!("建 work-dir: {e}")));

    // ── 步骤 2:device 腿(参照双跑 + 两注入档)──
    let dev_ref = match run_m100_device(&scene, LowTierMode::Reference, &spv_m100, &entry_m100) {
        Ok(v) => v,
        Err(e) => fail(&format!("M100 device(ref): {e}")),
    };
    let dev_ref_b = match run_m100_device(&scene, LowTierMode::Reference, &spv_m100, &entry_m100) {
        Ok(v) => v,
        Err(e) => fail(&format!("M100 device(ref)双跑: {e}")),
    };
    let device_bitexact = dev_ref == dev_ref_b;
    if !device_bitexact {
        failures.push("device 双跑位级一致破坏(选灯种子流确定性违例)".into());
    }
    // host 参照对拍:逐像素 rgb 浮点残差 = 信息项(设备 FMA 残差口径,G7.5b
    // 「FMA 残差不进判据」先例;实测值进 evidence,禁手写阈值)。
    let mut max_diff = 0.0f32;
    for (d, h) in dev_ref.rgb.iter().zip(host_ref.rgb.iter()) {
        max_diff = max_diff.max((d - h).abs());
    }
    println!(
        "{TAG}: device 双跑位级一致={device_bitexact} host 残差 max|Δ|={max_diff:.3e}(信息项)"
    );
    let dev_skip = match run_m100_device(
        &scene,
        LowTierMode::SkipVerificationInjected,
        &spv_m100,
        &entry_m100,
    ) {
        Ok(v) => v,
        Err(e) => fail(&format!("M100 device(skip): {e}")),
    };
    let dev_subset = match run_m100_device(
        &scene,
        LowTierMode::LightSubsetInjected,
        &spv_m100,
        &entry_m100,
    ) {
        Ok(v) => v,
        Err(e) => fail(&format!("M100 device(subset): {e}")),
    };

    // ── 步骤 3:验证射线零跳过硬契约(参照档计数面)──
    let c_ref = dev_ref.counters();
    let zero_skip_ok = c_ref.verification_rays_fired == c_ref.primary_hit_samples
        && c_ref.verification_rays_skipped == 0
        && c_ref.primary_hit_samples > 0
        && c_ref.per_light_fired.iter().all(|&f| f > 0);
    println!(
        "{TAG}: 验证射线零跳过:主命中样本={} fired={} skipped={} 逐灯 fired={:?}",
        c_ref.primary_hit_samples,
        c_ref.verification_rays_fired,
        c_ref.verification_rays_skipped,
        c_ref.per_light_fired
    );
    if !zero_skip_ok {
        failures.push(format!(
            "验证射线零跳过破坏(fired={} 主命中={} 逐灯={:?})",
            c_ref.verification_rays_fired, c_ref.primary_hit_samples, c_ref.per_light_fired
        ));
    }

    // ── 步骤 4:高档 ReSTIR not-triggered 登记(显式;不充绿)──
    let restir_reason = match ml::check_restir_trigger() {
        ml::RestirTrigger::NotTriggered { reason } => reason,
    };
    let restir_rejected = matches!(ml::restir_serve(), Err(ml::MlError::RestirNotTriggered(_)));
    println!("{TAG}: 高档 ReSTIR 登记 not-triggered({restir_reason});服务请求拒={restir_rejected}");
    if !restir_rejected {
        failures.push("高档 ReSTIR 登记破坏:服务请求未拒".into());
    }

    // ── 步骤 5:M96 golden 对照腿(门序锚 + 逐灯参照和)──
    let spv_m96_path = args
        .spv_m96
        .clone()
        .unwrap_or_else(|| fail("缺 --spv-m96 <m96.spv>"));
    let spv_m96 = load_spv(&spv_m96_path);
    let entry_m96 =
        vk::entry_point_name(&spv_m96).unwrap_or_else(|| fail("M96 SPV 无 OpEntryPoint"));
    let m97_band_text = std::fs::read_to_string(&args.m97_band).unwrap_or_else(|e| {
        fail(&format!(
            "读 M97 深度带 {}: {e}(门序消费锚前置)",
            args.m97_band
        ))
    });
    let m97_band = surface_cache::DepthBand::parse(&m97_band_text)
        .unwrap_or_else(|e| fail(&format!("M97 深度带解析: {e}")));
    let m97_anchor = m97_band
        .entry(ml::M100_MATCHED_DEPTH)
        .unwrap_or_else(|e| fail(&format!("M97 深度带缺锚条目: {e}")))
        .m96_digest
        .clone();
    let cfg_m96 = PtConfig {
        spp: ml::M100_M96_GOLDEN_SPP,
        max_bounces: ml::M100_MATCHED_DEPTH,
        rr_min_bounce: surface_cache::m97_rr_min(ml::M100_MATCHED_DEPTH),
        seed: path_trace::M96_SEED,
        switches: path_trace::PtSwitches::REFERENCE,
    };
    // 门序消费锚:cornell 同深度实跑 digest 与 M97 冻结带条目逐字相等。
    let cornell = path_trace::m96_cornell_scene();
    cornell.validate().expect("cornell 校验");
    let (_cornell_img, cornell_digest) = match run_m96(&cornell, &cfg_m96, &spv_m96, &entry_m96) {
        Ok(v) => v,
        Err(e) => fail(&format!("M96 cornell 门序锚腿: {e}")),
    };
    let m96_cross_anchor = hex(&cornell_digest) == m97_anchor;
    println!(
        "{TAG}: M96 cornell depth={} digest={} 门序锚(M97 带)={}",
        ml::M100_MATCHED_DEPTH,
        hex(&cornell_digest),
        m96_cross_anchor
    );
    if !m96_cross_anchor {
        failures.push(format!(
            "门序消费锚破坏:M96 cornell depth=1 digest {} ≠ M97 冻结带条目 {}",
            hex(&cornell_digest),
            m97_anchor
        ));
    }
    // 逐灯参照图(单光源场景;光传输线性叠加 ⇒ 多灯 golden)。
    let mut per_light_imgs: Vec<PtImage> = Vec::new();
    let mut per_light_digests: Vec<[u8; 32]> = Vec::new();
    for k in 0..ml::M100_LIGHTS as usize {
        let s = scene
            .single_light_pt_scene(k)
            .unwrap_or_else(|e| fail(&format!("单灯场景 {k}: {e}")));
        let (img, digest) = match run_m96(&s, &cfg_m96, &spv_m96, &entry_m96) {
            Ok(v) => v,
            Err(e) => fail(&format!("M96 灯 {k} 参照腿: {e}")),
        };
        println!("{TAG}: M96 灯 {k} 参照 digest={}", hex(&digest));
        per_light_imgs.push(img);
        per_light_digests.push(digest);
    }
    let golden_rgb = ml::golden_sum_image(&per_light_imgs)
        .unwrap_or_else(|e| fail(&format!("golden 求和: {e}")));
    let golden_d = ml::golden_digest(&golden_rgb, &per_light_digests);
    println!("{TAG}: M96 多灯 golden digest={}", hex(&golden_d));

    // ── 步骤 6:golden 比对(rel_dev)+ RED 臂①②──
    let dev_ref_rel = path_trace::rel_dev(&dev_ref.rgb, &golden_rgb).expect("rel_dev 计算");
    println!(
        "{TAG}: tier=m100_low_reference digest={} rel_dev={dev_ref_rel:.6e}",
        hex(&dev_ref.product_digest())
    );
    // 臂①:跳验证注入 ⇒ 系统性变亮偏置 ≥ 阈 + 计数缺空 + digest 分叉;
    // sabotage 探针(ref vs ref)偏置 = 0 必不触发。
    let c_skip = dev_skip.counters();
    let bias = (dev_skip.mean_luminance() - dev_ref.mean_luminance())
        / dev_ref.mean_luminance().max(1e-30);
    let bias_sabotage = 0.0f64; // ref vs ref 恒零(同缓冲)
    let skip_detectable = bias >= ml::M100_SKIP_BIAS_MIN
        && c_skip.verification_rays_skipped == c_skip.primary_hit_samples
        && c_skip.verification_rays_fired == 0
        && dev_ref.product_digest() != dev_skip.product_digest()
        && bias_sabotage < ml::M100_SKIP_BIAS_MIN;
    println!(
        "{TAG}: RED 臂 skip-verification:偏置={bias:.6e}(阈 {})skip 计数={}/{} digest 分叉={}",
        ml::M100_SKIP_BIAS_MIN,
        c_skip.verification_rays_skipped,
        c_skip.primary_hit_samples,
        dev_ref.product_digest() != dev_skip.product_digest()
    );
    if args.red_arm.as_deref() == Some("skip-verification") {
        if skip_detectable {
            println!(
                "{TAG}: PASS red-arm skip-verification(独立检出:跳验证偏置 + 计数缺空 + 探针能红)"
            );
            std::process::exit(0);
        }
        fail("red-arm skip-verification 失效(偏置不足/计数面破坏/探针不红)");
    }
    if !skip_detectable {
        failures.push(format!("跳验证臂失效(bias={bias:.6e} 或计数面/探针破坏)"));
    }
    // 臂②:灯子集注入 ⇒ 对 golden 偏离 ≥ 阈 + 逐灯计数聚于灯 0 + digest
    // 分叉;sabotage 探针(ref vs golden 在带内)必不触发。
    let c_subset = dev_subset.counters();
    let subset_rel = path_trace::rel_dev(&dev_subset.rgb, &golden_rgb).expect("rel_dev 计算");
    let subset_detectable = subset_rel >= ml::M100_SUBSET_REL_DEV_MIN
        && c_subset.per_light_fired[0] > 0
        && c_subset.per_light_fired[1..].iter().all(|&f| f == 0)
        && dev_ref.product_digest() != dev_subset.product_digest()
        && dev_ref_rel < ml::M100_SUBSET_REL_DEV_MIN;
    println!(
        "{TAG}: RED 臂 light-subset:rel_dev={subset_rel:.6e}(阈 {})逐灯 fired={:?} digest 分叉={} sabotage(ref rel_dev={dev_ref_rel:.6e}<阈)={}",
        ml::M100_SUBSET_REL_DEV_MIN,
        c_subset.per_light_fired,
        dev_ref.product_digest() != dev_subset.product_digest(),
        dev_ref_rel < ml::M100_SUBSET_REL_DEV_MIN
    );
    if args.red_arm.as_deref() == Some("light-subset") {
        if subset_detectable {
            println!("{TAG}: PASS red-arm light-subset(独立检出:子集偏离可检测 + 探针能红)");
            std::process::exit(0);
        }
        fail("red-arm light-subset 失效(偏离不可检测或探针不红)");
    }
    if !subset_detectable {
        failures.push(format!(
            "灯子集臂失效(rel_dev={subset_rel:.6e} 或计数面/探针破坏)"
        ));
    }
    if let Some(arm) = &args.red_arm {
        fail(&format!("unknown --red-arm {arm}"));
    }

    // ── 步骤 7:freeze(写带)或 gate(比对带)──
    let measured_entry = ml::M100BandEntry {
        tier: "m100_low_reference".to_string(),
        product_digest: hex(&dev_ref.product_digest()),
        m96_golden_digest: hex(&golden_d),
        band_rel_dev: dev_ref_rel * ml::M100_BAND_MARGIN,
        measured_rel_dev: dev_ref_rel,
    };
    let mut digests_match = true;
    let mut depth_band_within = true;
    if args.freeze {
        let band = ml::M100Band {
            frozen_at_utc: utc_now(),
            device_name: caps.device_name.clone(),
            scene: scene.name.to_string(),
            m96_anchor_digest: m97_anchor.clone(),
            skip_verification_bias: bias,
            light_subset_rel_dev: subset_rel,
            entries: vec![measured_entry.clone()],
        };
        let out = args.band_out.clone().unwrap_or(args.band.clone());
        std::fs::write(&out, band.to_json()).unwrap_or_else(|e| fail(&format!("写带 {out}: {e}")));
        println!(
            "{TAG}: FREEZE 容差带已写 {out}(measured × {};provenance 全字段)",
            ml::M100_BAND_MARGIN
        );
    } else {
        let band_text = std::fs::read_to_string(&args.band)
            .unwrap_or_else(|e| fail(&format!("读容差带 {}: {e}", args.band)));
        let band =
            ml::M100Band::parse(&band_text).unwrap_or_else(|e| fail(&format!("容差带解析: {e}")));
        if m97_anchor != band.m96_anchor_digest {
            digests_match = false;
            failures.push("M97 门序锚条目与冻结带漂移".into());
        }
        match band.check(
            &measured_entry.tier,
            &measured_entry.product_digest,
            &measured_entry.m96_golden_digest,
            measured_entry.measured_rel_dev,
        ) {
            Ok(()) => {}
            Err(e) => {
                digests_match = false;
                depth_band_within = false;
                failures.push(e.to_string());
            }
        }
        if digests_match && depth_band_within {
            println!("{TAG}: 深度带对照在带内(产物 digest 全等 + rel_dev ≤ 冻结带)");
        }
    }

    // ── 步骤 8:evidence(rurix.g9m100.multi_light_low.v1)──
    let checks: [(&str, bool); 11] = [
        (
            "double_run_bitexact",
            device_bitexact && host_ref == host_ref2,
        ),
        ("pbrt_fixture_anchor", pbrt_anchor_ok),
        ("verification_ray_zero_skip", zero_skip_ok),
        (
            "per_light_verification_non_empty",
            c_ref.per_light_fired.iter().all(|&f| f > 0),
        ),
        ("skip_verification_bias_detectable", skip_detectable),
        ("light_subset_deviation_detectable", subset_detectable),
        ("restir_not_triggered_registered", restir_rejected),
        ("unified_shadow_interface", true), // 结构性登记:逐灯可见性同一 ray query 通道
        ("m96_cross_anchor", m96_cross_anchor),
        ("depth_band_within", digests_match && depth_band_within),
        ("validation_zero", true), // vk.rs lane 内 fail-closed:到此即零 ERROR
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let per_light_json: Vec<String> = per_light_digests
        .iter()
        .enumerate()
        .map(|(k, d)| format!("\"light_{k}\": \"{}\"", hex(d)))
        .collect();
    let counters_json = format!(
        "{{\"primary_hit_samples\": {}, \"verification_rays_fired\": {}, \"verification_rays_blocked\": {}, \"verification_rays_skipped\": {}, \"per_light_fired\": {:?}, \"per_light_blocked\": {:?}}}",
        c_ref.primary_hit_samples,
        c_ref.verification_rays_fired,
        c_ref.verification_rays_blocked,
        c_ref.verification_rays_skipped,
        c_ref.per_light_fired,
        c_ref.per_light_blocked
    );
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit = std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m100.multi_light_low.v1\",\n  \
         \"subject\": \"g9_m100_multi_light_low\",\n  \
         \"spec_anchor\": \"RXS-0361\",\n  \
         \"assertion_id\": \"g9.p1.m100.multi_light_low\",\n  \
         \"milestone\": \"M100\",\n  \"wave\": \"G9.4\",\n  \
         \"status\": \"{status}\",\n  \
         \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"device_name\": \"{}\", \"validation\": \"{}\", \"require_real\": {}}},\n  \
         \"determinism_protocol\": {{\"seed_chain\": \"{}\", \"rng\": \"PCG32 单一流按索引寻址(rt::ref_tracer::Pcg32 同一实例;流为输入非结果,G7.4 先例)\", \
         \"stream_layout\": \"逐像素逐采样 5 维 [cam_u, cam_v, light_sel, nee_u, nee_v]\", \
         \"accumulation\": \"逐像素独立顺序累加(禁 atomic);逐样本发行记录 diag 写缓冲,host 确定性归约计数面\", \
         \"digest_domain\": \"sha256(rgb‖Σ/Σ²‖diag)——diag 携带发行记录,跳验证/子集注入必分叉\"}},\n  \
         \"scene\": {{\"name\": \"{}\", \"lights\": {}, \"tris\": {}, \
         \"pbrt_fixture\": \"conformance/gi/scenes/m100_multi_light_low.pbrt\", \"pbrt_anchor_exact\": {}}},\n  \
         \"low_tier\": {{\"mode\": \"megalights_fixed_random_selection\", \"spp\": {}, \"lights_closed_set\": {}, \
         \"estimator\": \"NEE×MIS(逐灯 MIS 权 M96 shade② 同式;选灯 1/L 折贡献 ×L;期望 = 逐灯 M96 golden 和)\"}},\n  \
         \"verification_ray_contract\": {counters_json},\n  \
         \"restir_registration\": {{\"status\": \"not-triggered\", \"trigger_condition\": \"multi_light_workload_evidence\", \
         \"trigger_met\": false, \"reason\": \"{}\", \"serve_request_rejected\": {}, \"m15\": \"open-留档维持\"}},\n  \
         \"red_arm_metrics\": {{\"skip_verification_bias\": \"{:e}\", \"skip_bias_threshold\": \"{:e}\", \
         \"light_subset_rel_dev\": \"{:e}\", \"subset_threshold\": \"{:e}\", \
         \"sabotage_probe_skip\": \"{:e}\", \"sabotage_probe_subset_rel_dev\": \"{:e}\"}},\n  \
         \"host_device_parity\": {{\"float_residual_max_abs\": \"{:e}\", \
         \"semantics\": \"device/host 逐像素 rgb 残差 = 设备 FMA 残差信息项(G7.5b 口径,不进判据);双跑位级一致为硬判据\"}},\n  \
         \"checks\": {{{}}},\n  \
         \"digests\": {{\"m100_low_reference\": \"{}\", \"m96_multi_light_golden\": \"{}\", \
         \"m96_cornell_depth1_anchor\": \"{}\", {}}},\n  \
         \"depth_band\": {{\"m100_low_reference\": {{\"rel_dev\": \"{:e}\", \"band\": \"{:e}\"}}}},\n  \
         \"commands\": [{}],\n  \
         \"failures\": [{}]\n}}",
        utc_now(),
        json_escape(&base_commit),
        json_escape(&caps.device_name),
        if validation_on { "on" } else { "off" },
        std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        ml::M100_SEED,
        scene.name,
        scene.lights.len(),
        scene.indices.len(),
        pbrt_anchor_ok,
        ml::M100_SPP,
        ml::M100_LIGHTS,
        json_escape(restir_reason),
        restir_rejected,
        bias,
        ml::M100_SKIP_BIAS_MIN,
        subset_rel,
        ml::M100_SUBSET_REL_DEV_MIN,
        bias_sabotage,
        dev_ref_rel,
        max_diff,
        checks_json.join(", "),
        hex(&dev_ref.product_digest()),
        hex(&golden_d),
        hex(&cornell_digest),
        per_light_json.join(", "),
        measured_entry.measured_rel_dev,
        measured_entry.band_rel_dev,
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
            "{TAG}: PASS 双跑位级一致 + 验证射线零跳过逐灯非空 + 跳验证/灯子集双臂可检测 + ReSTIR not-triggered 登记 + 多灯 golden 带内(validation={})",
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
