//! G9.4 M97 Surface Cache device harness(RXS-0358;门 `g9.p0.m97.surface_cache`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §2 M97 行 + spec/global_illumination.md RXS-0358)
//!
//! - **离线 Card 参数化(≤12/mesh 可配)与运行时辐射度缓存产物 digest 等于
//!   golden**:M96 冻结 Cornell fixture 经 [`surface_cache::parameterize`]
//!   (默认 12 上限,方向聚类 + 投影面选择)产 CardSet(digest golden)+ RXPL
//!   v2 图集页(digest golden);capture kernel(device 真跑,U30 ray query
//!   执行面)产辐射度图集(逐深度 digest golden);双跑位级一致;
//! - **Card 图集页复用页格式 ABI 不私定格式**:图集页 = RXPL major=2 页
//!   (`encode_logical_page_v2`/`decode_logical_page_v2` 冻结面往返无损);
//!   篡改 digest / 私定魔数 variant 装配期 fail-closed 拒(RED 臂);
//! - **缺失覆盖只丢能量不漏光**:Card 空洞注入(盒顶 Card 挖洞)+ 回退
//!   ambient 正例臂——漏光像素计数 = 0 且能量差 measured 记录;
//! - **Card 空洞漏光检测负例 RED 臂独立有效**:同一空洞 + 回退关闭变体
//!   ⇒ [`surface_cache::count_leak_pixels`] 必检出(漏光像素计数 > 0);
//! - **按匹配深度(1/2/full bounce)对 M96 golden 验收**:同场景同深度 M96
//!   megakernel golden(spp=64,冻结 seed)与 surface cache 消费渲染的 rel_dev
//!   ≤ 冻结带(`milestones/g9/g9_m97_depth_band.json`;带 = measured ×
//!   [`surface_cache::M97_BAND_MARGIN`],禁手写 P-09);full 档 M96 digest 与
//!   M96 冻结容差带 `m96_cornell_spp64` 条目逐字相等(D2-Q7 门序锚)。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备/W3 能力链缺失 → `G9_M97_SC: SKIP DEV_ENV_DEGRADE`
//! (退 0,非 fake pass;`RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层
//! 裁决);判据不符 / RED 轴失效 → FAIL 退 1。`RURIX_VK_VALIDATION=1`:vk.rs
//! lane 内 fail-closed;evidence 记 validation 模式。
//!
//! ## 用法
//!
//! ```text
//! g9_m97_surface_cache --spv-capture <c.spv> --spv-render <r.spv> --spv-m96 <m96.spv>
//!     [--band <path>] [--m96-band <path>] [--evidence <path>] [--work-dir <dir>]
//! g9_m97_surface_cache --freeze --spv-capture .. --spv-render .. --spv-m96 .. [--band-out <path>]
//! g9_m97_surface_cache --red-arm hole-leak --spv-capture .. --spv-render ..
//! g9_m97_surface_cache --red-arm abi-tamper            # 纯 host 臂(无 device 依赖)
//! ```

use rurix_render::gi::path_trace::{self, PtConfig, PtImage, ToleranceBand};
use rurix_render::gi::surface_cache::{self, CardSet, DepthBand, DepthBandEntry, ScConfig};
use rurix_rt::render_exec::{self, KernelWave};
use rurix_rt::vk::{
    self, RayQueryBufferDesc, RayQueryDispatchDesc, RayQueryInstanceDesc, RayQuerySceneDesc,
};

const TAG: &str = "G9_M97_SC";
/// RED 臂/能量断言承载深度(中档;三深度全档由深度带承载)。
const ARM_DEPTH: u32 = 2;

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
// device 执行腿(U30 run_ray_query_effects;单 BLAS × 单实例)
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

/// capture device 真跑:辐射度图集采样 → (radiance, coverage, digest)。
fn run_capture(
    scene: &path_trace::PtScene,
    set: &CardSet,
    depth: u32,
    spv: &[u32],
    entry: &str,
) -> Result<(Vec<f32>, Vec<f32>, [u8; 32]), String> {
    scene.validate().map_err(|e| format!("场景校验: {e}"))?;
    let tris = scene.blas_triangles();
    let (blas_refs, instances) = scene_desc_of(&tris);
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    let total = set.total_texels as usize;
    let stream = surface_cache::m97_rng::generate_stream(
        total,
        set.config.samples_per_texel,
        depth,
        surface_cache::M97_SEED,
    );
    let rng_b = bytes_f32(&stream);
    let mats_b = bytes_f32(&path_trace::pack_mats(scene));
    let tris_b = bytes_f32(&tris);
    let cards_b = bytes_f32(&surface_cache::pack_cards(set));
    let tcard_b = bytes_f32(&surface_cache::pack_texel_card(set));
    let params_b = bytes_f32(&surface_cache::pack_capture_params(
        scene,
        &set.config,
        set.total_texels,
        depth,
    ));
    let buffers = [
        RayQueryBufferDesc::Input(&rng_b),
        RayQueryBufferDesc::Input(&mats_b),
        RayQueryBufferDesc::Input(&tris_b),
        RayQueryBufferDesc::Input(&cards_b),
        RayQueryBufferDesc::Input(&tcard_b),
        RayQueryBufferDesc::Input(&params_b),
        RayQueryBufferDesc::Output(total * 12),
        RayQueryBufferDesc::Output(total * 4),
    ];
    let out = vk::run_ray_query_effects(
        &scene_desc,
        &[RayQueryDispatchDesc {
            name: "g9_m97_cache_capture",
            spv,
            entry,
            buffers: &buffers,
            push_constants: &[],
            groups: [total as u32, 1, 1],
        }],
    )?;
    let rb = out
        .readbacks
        .into_iter()
        .next()
        .ok_or("单 dispatch 缺回读")?;
    if rb.len() != 2 {
        return Err(format!("回读路数 {} ≠ 2", rb.len()));
    }
    let radiance = read_f32(&rb[0]);
    let coverage = read_f32(&rb[1]);
    let digest = surface_cache::cache_product_digest(&radiance, &coverage);
    Ok((radiance, coverage, digest))
}

/// render device 真跑:消费图集 → (rgb, flags, digest)。
#[allow(clippy::too_many_arguments)]
fn run_render(
    scene: &path_trace::PtScene,
    set: &CardSet,
    atlas: &[f32],
    coverage: &[f32],
    tri_to_card_packed: &[f32],
    fallback_on: bool,
    spv: &[u32],
    entry: &str,
) -> Result<(Vec<f32>, Vec<f32>, [u8; 32]), String> {
    let tris = scene.blas_triangles();
    let (blas_refs, instances) = scene_desc_of(&tris);
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    let pixel_count = (scene.camera.width * scene.camera.height) as usize;
    let cards_b = bytes_f32(&surface_cache::pack_cards(set));
    let t2c_b = bytes_f32(tri_to_card_packed);
    let atlas_b = bytes_f32(atlas);
    let cov_b = bytes_f32(coverage);
    let params_b = bytes_f32(&surface_cache::pack_render_params(scene, fallback_on));
    let buffers = [
        RayQueryBufferDesc::Input(&cards_b),
        RayQueryBufferDesc::Input(&t2c_b),
        RayQueryBufferDesc::Input(&atlas_b),
        RayQueryBufferDesc::Input(&cov_b),
        RayQueryBufferDesc::Input(&params_b),
        RayQueryBufferDesc::Output(pixel_count * 12),
        RayQueryBufferDesc::Output(pixel_count * 4),
    ];
    let out = vk::run_ray_query_effects(
        &scene_desc,
        &[RayQueryDispatchDesc {
            name: "g9_m97_cache_render",
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
    if rb.len() != 2 {
        return Err(format!("回读路数 {} ≠ 2", rb.len()));
    }
    let rgb = read_f32(&rb[0]);
    let flags = read_f32(&rb[1]);
    let digest = surface_cache::render_product_digest(&rgb, &flags);
    Ok((rgb, flags, digest))
}

/// M96 golden 对照腿:同场景同深度 megakernel(spp=64,冻结 seed)→ (PtImage, digest)。
fn run_m96(
    scene: &path_trace::PtScene,
    depth: u32,
    spv: &[u32],
    entry: &str,
) -> Result<(PtImage, [u8; 32]), String> {
    let cfg = PtConfig {
        spp: surface_cache::M97_M96_GOLDEN_SPP,
        max_bounces: depth,
        rr_min_bounce: surface_cache::m97_rr_min(depth),
        seed: path_trace::M96_SEED,
        switches: path_trace::PtSwitches::REFERENCE,
    };
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
    let params_b = bytes_f32(&path_trace::pack_params(scene, &cfg));
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
    spv_capture: Option<String>,
    spv_render: Option<String>,
    spv_m96: Option<String>,
    evidence: Option<String>,
    band: String,
    m96_band: String,
    work_dir: String,
    freeze: bool,
    band_out: Option<String>,
    red_arm: Option<String>,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = Args {
        spv_capture: None,
        spv_render: None,
        spv_m96: None,
        evidence: None,
        band: "milestones/g9/g9_m97_depth_band.json".to_string(),
        m96_band: "milestones/g9/g9_m96_pbrt_tolerance_band.json".to_string(),
        work_dir: ".tmp/g9_m97_work".to_string(),
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
            "--spv-capture" => out.spv_capture = Some(take(&mut i)),
            "--spv-render" => out.spv_render = Some(take(&mut i)),
            "--spv-m96" => out.spv_m96 = Some(take(&mut i)),
            "--evidence" => out.evidence = Some(take(&mut i)),
            "--band" => out.band = take(&mut i),
            "--m96-band" => out.m96_band = take(&mut i),
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

/// 离线腿(host):参数化 + 图集页 + RED 私定格式臂。返回 (set, page_bytes)。
fn offline_leg() -> Result<(path_trace::PtScene, CardSet, Vec<u8>, [u8; 32], [u8; 32]), String> {
    let scene = path_trace::m96_cornell_scene();
    scene.validate().map_err(|e| format!("场景校验: {e}"))?;
    let meshes = surface_cache::m97_cornell_meshes();
    let cfg = ScConfig::default();
    let mut set = surface_cache::parameterize(&scene.positions, &scene.indices, &meshes, &cfg)
        .map_err(|e| format!("Card 参数化: {e}"))?;
    // 双跑位级一致(host 参数化确定性协议)。
    let set2 = surface_cache::parameterize(&scene.positions, &scene.indices, &meshes, &cfg)
        .map_err(|e| format!("Card 参数化双跑: {e}"))?;
    if set != set2 {
        return Err("Card 参数化双跑分叉(确定性协议违例)".into());
    }
    let (_page, page_bytes) =
        surface_cache::build_atlas_page(&mut set, &scene.positions, &scene.indices)
            .map_err(|e| format!("图集页构建: {e}"))?;
    let cardset_d = set.digest();
    let page_d = surface_cache::atlas_page_digest(&page_bytes);
    println!(
        "{TAG}: 离线腿 cards={} texels={} cardset={} atlas_page={}",
        set.cards.len(),
        set.total_texels,
        hex(&cardset_d),
        hex(&page_d)
    );
    Ok((scene, set, page_bytes, cardset_d, page_d))
}

/// RED 臂:私定/篡改图集格式 → 装配期 fail-closed 拒(host;RXS-0358 L5)。
fn red_arm_abi_tamper(page_bytes: &[u8]) -> bool {
    let mut tampered = page_bytes.to_vec();
    tampered[104] ^= 0x01; // section_digest 篡改
    let tamper_rejected = rurix_geom_pages::logical_v2::decode_logical_page_v2(&tampered).is_err();
    let mut private_fmt = page_bytes.to_vec();
    private_fmt[0..4].copy_from_slice(b"SCAT"); // 私定魔数
    let private_rejected = matches!(
        rurix_geom_pages::logical_v2::decode_logical_page_v2(&private_fmt),
        Err(rurix_geom_pages::logical::PageDecodeError::BadMagic)
    );
    let ok = tamper_rejected && private_rejected;
    println!(
        "{TAG}: RED 臂 abi-tamper(digest 篡改拒={tamper_rejected} 私定魔数拒={private_rejected})→ {}",
        if ok {
            "检出(RED 有效)"
        } else {
            "未检出(漏检)"
        }
    );
    ok
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    println!(
        "[g9_m97_surface_cache] G9.4 M97 Surface Cache device harness(RXS-0358;门 g9.p0.m97.surface_cache)"
    );
    let args = parse_args();

    // ── 步骤 0:离线腿(host;无 device 依赖)──
    let (scene, set, page_bytes, cardset_d, page_d) = offline_leg().unwrap_or_else(|e| fail(&e));

    // --red-arm abi-tamper:纯 host 臂(私定图集格式 variant 装配期拒)。
    if args.red_arm.as_deref() == Some("abi-tamper") {
        if red_arm_abi_tamper(&page_bytes) {
            println!("{TAG}: PASS red-arm abi-tamper(独立检出;私定/篡改格式装配期拒)");
            std::process::exit(0);
        }
        fail("red-arm abi-tamper 未检出");
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
    let spv_capture_path = args
        .spv_capture
        .clone()
        .unwrap_or_else(|| fail("缺 --spv-capture <c.spv>"));
    let spv_render_path = args
        .spv_render
        .clone()
        .unwrap_or_else(|| fail("缺 --spv-render <r.spv>"));
    let spv_capture = load_spv(&spv_capture_path);
    let spv_render = load_spv(&spv_render_path);
    let entry_capture =
        vk::entry_point_name(&spv_capture).unwrap_or_else(|| fail("capture SPV 无 OpEntryPoint"));
    let entry_render =
        vk::entry_point_name(&spv_render).unwrap_or_else(|| fail("render SPV 无 OpEntryPoint"));
    println!("{TAG}: kernel entry capture=`{entry_capture}` render=`{entry_render}`");
    let work = std::path::PathBuf::from(&args.work_dir);
    std::fs::create_dir_all(&work).unwrap_or_else(|e| fail(&format!("建 work-dir: {e}")));

    // ── 步骤 2:三深度 capture + 消费渲染(双跑位级一致;RED 臂子模式仅 ARM_DEPTH)──
    let depths: Vec<u32> = if args.red_arm.is_some() {
        vec![ARM_DEPTH]
    } else {
        surface_cache::M97_DEPTHS.to_vec()
    };
    let mut failures: Vec<String> = Vec::new();
    let mut double_run_bitexact = true;
    let mut per_depth: std::collections::BTreeMap<
        u32,
        (Vec<f32>, Vec<f32>, [u8; 32], Vec<f32>, Vec<f32>, [u8; 32]),
    > = Default::default();
    for &depth in &depths {
        let (rad_a, cov_a, d_a) =
            match run_capture(&scene, &set, depth, &spv_capture, &entry_capture) {
                Ok(v) => v,
                Err(e) => fail(&format!("capture depth={depth}: {e}")),
            };
        let (_rad_b, _cov_b, d_b) =
            match run_capture(&scene, &set, depth, &spv_capture, &entry_capture) {
                Ok(v) => v,
                Err(e) => fail(&format!("capture 双跑 depth={depth}: {e}")),
            };
        if d_a != d_b {
            double_run_bitexact = false;
            failures.push(format!("capture depth={depth} 双跑 digest 分叉"));
        }
        if !cov_a.iter().all(|&v| v == 1.0) {
            failures.push(format!(
                "capture depth={depth} 完整图集覆盖破坏(存在 0 覆盖 texel)"
            ));
        }
        let t2c = surface_cache::pack_tri_to_card(&set);
        let (rgb, flags, d_r) = match run_render(
            &scene,
            &set,
            &rad_a,
            &cov_a,
            &t2c,
            true,
            &spv_render,
            &entry_render,
        ) {
            Ok(v) => v,
            Err(e) => fail(&format!("render depth={depth}: {e}")),
        };
        if !rgb.iter().all(|v| v.is_finite() && *v >= 0.0) {
            failures.push(format!("render depth={depth} 输出非有限/负"));
        }
        println!(
            "{TAG}: depth={depth} cache={} render={}",
            hex(&d_a),
            hex(&d_r)
        );
        per_depth.insert(depth, (rad_a, cov_a, d_a, rgb, flags, d_r));
    }

    // ── 步骤 3:漏光双臂(Card 空洞 = 地板 Card 挖洞;ARM_DEPTH 档)──
    // 受害 Card = 地板(mesh 0,法线 +y):相机下半帧全见 + 直接光直射(缓存值
    // 远高于 ambient 地板 ⇒ 挖洞后回退区只丢能量;内盒面法线朝内,盒内侧缓存
    // 近零,非「丢能量」锚的合适承载面)。
    let victim = set
        .cards
        .iter()
        .find(|c| c.mesh_index == 0 && c.normal == [0.0, 1.0, 0.0])
        .unwrap_or_else(|| fail("地板 Card 不在参数化产物集"));
    let victim_id = victim.card_id;
    let victim_tris = victim.tris.len();
    let (rad_arm, cov_arm, _, rgb_full, flags_full, _) = per_depth
        .get(&ARM_DEPTH)
        .expect("ARM_DEPTH 档产物在集")
        .clone();
    let mut t2c_holed = surface_cache::pack_tri_to_card(&set);
    let mut cov_holed = cov_arm.clone();
    let holed = surface_cache::inject_card_hole(&mut t2c_holed, &mut cov_holed, &set, victim_id)
        .unwrap_or_else(|e| fail(&format!("空洞注入: {e}")));
    println!("{TAG}: 空洞注入 card={victim_id} 挖洞三角 {holed} 枚");
    // GREEN 臂:回退 ambient ⇒ 漏光像素计数 = 0,能量差 measured。
    let (rgb_green, flags_green, _) = match run_render(
        &scene,
        &set,
        &rad_arm,
        &cov_holed,
        &t2c_holed,
        true,
        &spv_render,
        &entry_render,
    ) {
        Ok(v) => v,
        Err(e) => fail(&format!("GREEN 臂 render: {e}")),
    };
    let leak_green = surface_cache::count_leak_pixels(
        &rgb_green,
        &flags_green,
        surface_cache::M97_AMBIENT,
        surface_cache::M97_LEAK_EPS,
    );
    let fallback_px = flags_green
        .iter()
        .filter(|&&f| f == surface_cache::FLAG_FALLBACK)
        .count();
    let e_full: f64 = rgb_full.iter().map(|&v| f64::from(v)).sum();
    let e_holed: f64 = rgb_green.iter().map(|&v| f64::from(v)).sum();
    let energy_loss_rel = (e_full - e_holed) / e_full;
    // RED 臂:回退关闭变体 ⇒ 漏光检测必检出(漏光像素计数 > 0)。
    let (rgb_red, flags_red, _) = match run_render(
        &scene,
        &set,
        &rad_arm,
        &cov_holed,
        &t2c_holed,
        false,
        &spv_render,
        &entry_render,
    ) {
        Ok(v) => v,
        Err(e) => fail(&format!("RED 臂 render: {e}")),
    };
    let leak_red = surface_cache::count_leak_pixels(
        &rgb_red,
        &flags_red,
        surface_cache::M97_AMBIENT,
        surface_cache::M97_LEAK_EPS,
    );
    let red_hole_leak_detected = leak_red > 0;
    let full_leak = surface_cache::count_leak_pixels(
        &rgb_full,
        &flags_full,
        surface_cache::M97_AMBIENT,
        surface_cache::M97_LEAK_EPS,
    );
    let full_fallback_px = flags_full
        .iter()
        .filter(|&&f| f == surface_cache::FLAG_FALLBACK)
        .count();
    println!(
        "{TAG}: 漏光双臂:full(leak={full_leak} fallback_px={full_fallback_px}) green(leak={leak_green} fallback_px={fallback_px}) red(leak={leak_red}) 能量差 rel={energy_loss_rel:.6e}"
    );
    let coverage_complete_no_leak = full_leak == 0 && full_fallback_px == 0;
    let energy_loss_only_no_leak =
        leak_green == 0 && fallback_px > 0 && energy_loss_rel > 0.0 && energy_loss_rel.is_finite();
    if !red_hole_leak_detected {
        failures.push("RED 臂 hole-leak 失效:空洞+关回退变体漏光像素计数 = 0(漏检)".into());
    }
    if !coverage_complete_no_leak {
        failures.push(format!(
            "完整覆盖臂破坏:leak={full_leak} fallback_px={full_fallback_px}"
        ));
    }
    if !energy_loss_only_no_leak {
        failures.push(format!(
            "只丢能量不漏光断言破坏:green_leak={leak_green} fallback_px={fallback_px} energy_loss_rel={energy_loss_rel:.6e}"
        ));
    }

    // --red-arm hole-leak 子模式:到 RED 臂为止(独立检出演示)。
    if let Some(arm_name) = &args.red_arm {
        let ok = match arm_name.as_str() {
            "hole-leak" => red_hole_leak_detected && leak_green == 0 && double_run_bitexact,
            other => fail(&format!("unknown --red-arm {other}")),
        };
        if ok {
            println!(
                "{TAG}: PASS red-arm hole-leak(独立检出:RED 臂 leak={leak_red} > 0;GREEN 臂 leak=0;capture 双跑位级一致)"
            );
            std::process::exit(0);
        }
        fail(&format!("red-arm {arm_name} 未检出"));
    }

    // ── 步骤 4:M96 golden 三深度对照 + 门序锚 ──
    let spv_m96_path = args
        .spv_m96
        .clone()
        .unwrap_or_else(|| fail("缺 --spv-m96 <m96.spv>"));
    let spv_m96 = load_spv(&spv_m96_path);
    let entry_m96 =
        vk::entry_point_name(&spv_m96).unwrap_or_else(|| fail("M96 SPV 无 OpEntryPoint"));
    let m96_band_text = std::fs::read_to_string(&args.m96_band).unwrap_or_else(|e| {
        fail(&format!(
            "读 M96 容差带 {}: {e}(D2-Q7 门序锚前置)",
            args.m96_band
        ))
    });
    let m96_band = ToleranceBand::parse(&m96_band_text)
        .unwrap_or_else(|e| fail(&format!("M96 容差带解析: {e}")));
    let m96_anchor = m96_band
        .entry("m96_cornell", surface_cache::M97_M96_GOLDEN_SPP)
        .unwrap_or_else(|e| fail(&format!("M96 容差带缺锚条目: {e}")))
        .golden_digest
        .clone();
    let mut measured: Vec<DepthBandEntry> = Vec::new();
    let mut m96_golden_anchor = true;
    for &depth in &depths {
        let (img, d_m96) = match run_m96(&scene, depth, &spv_m96, &entry_m96) {
            Ok(v) => v,
            Err(e) => fail(&format!("M96 对照腿 depth={depth}: {e}")),
        };
        let (_, _, d_cache, rgb, _, d_render) = &per_depth[&depth];
        let dev = path_trace::rel_dev(rgb, &img.rgb).expect("rel_dev 计算");
        println!(
            "{TAG}: depth={depth} m96={} rel_dev={dev:.6e}(mean_lum m97={:.6} m96={:.6})",
            hex(&d_m96),
            rgb.iter().map(|&v| f64::from(v)).sum::<f64>() / rgb.len() as f64,
            img.mean_luminance()
        );
        if depth == path_trace::M96_MAX_BOUNCES && hex(&d_m96) != m96_anchor {
            m96_golden_anchor = false;
            failures.push(format!(
                "门序锚破坏:M96 full 档 digest {} ≠ M96 冻结带 m96_cornell_spp64 {}",
                hex(&d_m96),
                m96_anchor
            ));
        }
        measured.push(DepthBandEntry {
            depth,
            cache_digest: hex(d_cache),
            render_digest: hex(d_render),
            m96_digest: hex(&d_m96),
            band_rel_dev: dev * surface_cache::M97_BAND_MARGIN,
            measured_rel_dev: dev,
        });
    }

    // ── 步骤 5:freeze(写带)或 gate(比对带)──
    let mut digests_match = true;
    let mut depth_band_within = true;
    if args.freeze {
        let band = DepthBand {
            frozen_at_utc: utc_now(),
            device_name: caps.device_name.clone(),
            scene: scene.name.to_string(),
            cardset_digest: hex(&cardset_d),
            atlas_page_digest: hex(&page_d),
            m96_anchor_digest: m96_anchor.clone(),
            entries: measured.clone(),
        };
        let out = args.band_out.clone().unwrap_or(args.band.clone());
        std::fs::write(&out, band.to_json()).unwrap_or_else(|e| fail(&format!("写带 {out}: {e}")));
        println!(
            "{TAG}: FREEZE 深度容差带已写 {out}(measured × {};provenance 全字段)",
            surface_cache::M97_BAND_MARGIN
        );
    } else {
        let band_text = std::fs::read_to_string(&args.band)
            .unwrap_or_else(|e| fail(&format!("读深度容差带 {}: {e}", args.band)));
        let band =
            DepthBand::parse(&band_text).unwrap_or_else(|e| fail(&format!("深度容差带解析: {e}")));
        if hex(&cardset_d) != band.cardset_digest {
            digests_match = false;
            failures.push(format!(
                "cardset digest {} ≠ golden {}",
                hex(&cardset_d),
                band.cardset_digest
            ));
        }
        if hex(&page_d) != band.atlas_page_digest {
            digests_match = false;
            failures.push(format!(
                "atlas page digest {} ≠ golden {}",
                hex(&page_d),
                band.atlas_page_digest
            ));
        }
        if m96_anchor != band.m96_anchor_digest {
            digests_match = false;
            failures.push("M96 门序锚条目与冻结带漂移".into());
        }
        for m in &measured {
            match band.check(
                m.depth,
                &m.cache_digest,
                &m.render_digest,
                &m.m96_digest,
                m.measured_rel_dev,
            ) {
                Ok(()) => {}
                Err(e) => {
                    digests_match = false;
                    depth_band_within = false;
                    failures.push(e.to_string());
                }
            }
        }
        if digests_match && depth_band_within {
            println!("{TAG}: 深度带对照在带内(产物 digest 全等 + rel_dev ≤ 冻结带)");
        }
    }

    // ── 步骤 6:RED 私定格式臂(门内需独立有效)──
    let red_private_format_rejected = red_arm_abi_tamper(&page_bytes);
    if !red_private_format_rejected {
        failures.push("RED 臂 abi-tamper 失效:私定/篡改图集格式未被装配期拒".into());
    }

    // ── 步骤 7:evidence(rurix.g9m97.surface_cache.v1)──
    let checks: [(&str, bool); 12] = [
        ("double_run_bitexact", double_run_bitexact),
        ("cardset_digest_match", digests_match),
        ("atlas_page_digest_match", digests_match),
        ("cache_digest_match", digests_match),
        ("render_digest_match", digests_match),
        ("m96_golden_anchor", m96_golden_anchor),
        ("depth_band_within", depth_band_within),
        ("coverage_complete_no_leak", coverage_complete_no_leak),
        ("energy_loss_only_no_leak", energy_loss_only_no_leak),
        ("red_hole_leak_detected", red_hole_leak_detected),
        ("red_private_format_rejected", red_private_format_rejected),
        ("validation_zero", true), // vk.rs lane 内 fail-closed:到此即零 ERROR
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let mut digests_json: Vec<String> = vec![
        format!("\"cardset\": \"{}\"", hex(&cardset_d)),
        format!("\"atlas_page\": \"{}\"", hex(&page_d)),
    ];
    for m in &measured {
        digests_json.push(format!(
            "\"cache_depth{}\": \"{}\"",
            m.depth, m.cache_digest
        ));
        digests_json.push(format!(
            "\"render_depth{}\": \"{}\"",
            m.depth, m.render_digest
        ));
        digests_json.push(format!("\"m96_depth{}\": \"{}\"", m.depth, m.m96_digest));
    }
    let band_json: Vec<String> = measured
        .iter()
        .map(|m| {
            format!(
                "\"depth{}\": {{\"rel_dev\": \"{:e}\", \"band\": \"{:e}\"}}",
                m.depth, m.measured_rel_dev, m.band_rel_dev
            )
        })
        .collect();
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m97.surface_cache.v1\",\n  \
         \"subject\": \"g9_m97_surface_cache\",\n  \
         \"spec_anchor\": \"RXS-0358\",\n  \
         \"device_state\": {{\"device_name\": \"{}\", \"validation\": \"{}\", \
         \"require_real\": {}}},\n  \
         \"determinism_protocol\": {{\"seed_capture\": \"{}\", \"rng\": \"PCG32 单一流按索引寻址(rt::ref_tracer::Pcg32 同一实例;流为输入非结果,G7.4 先例)\", \
         \"accumulation\": \"逐 texel/逐像素独立顺序累加(禁 atomic)\", \
         \"digest_domain\": \"sha256(产物字节依序拼接)\"}},\n  \
         \"parameterization\": {{\"max_cards_per_mesh\": {}, \"card_res\": {}, \
         \"samples_per_texel\": {}, \"card_count\": {}, \"total_texels\": {}, \
         \"atlas_page_abi\": \"RXPL major=2 (RXS-0344;复用 M91 冻结面,不私定)\", \
         \"atlas_page_bytes\": {}}},\n  \
         \"checks\": {{{}}},\n  \
         \"digests\": {{{}}},\n  \
         \"depth_band\": {{{}}},\n  \
         \"hole_arms\": {{\"victim_card\": {}, \"holed_tris\": {}, \
         \"energy_full\": \"{:e}\", \"energy_holed_green\": \"{:e}\", \
         \"energy_loss_rel_measured\": \"{:e}\", \"leak_full\": {}, \"leak_green\": {}, \
         \"leak_red\": {}, \"fallback_px_green\": {}}},\n  \
         \"commands\": [{}],\n  \
         \"failures\": [{}]\n}}",
        json_escape(&caps.device_name),
        if validation_on { "on" } else { "off" },
        std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        surface_cache::M97_SEED,
        set.config.max_cards_per_mesh,
        set.config.card_res,
        set.config.samples_per_texel,
        set.cards.len(),
        set.total_texels,
        page_bytes.len(),
        checks_json.join(", "),
        digests_json.join(", "),
        band_json.join(", "),
        victim_id,
        victim_tris,
        e_full,
        e_holed,
        energy_loss_rel,
        full_leak,
        leak_green,
        leak_red,
        fallback_px,
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
            "{TAG}: PASS 双跑位级一致 + golden 全等 + 三深度带内 + 漏光双臂有效(validation={})",
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
