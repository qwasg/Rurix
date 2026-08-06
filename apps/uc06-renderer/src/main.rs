//! uc06-renderer — rurix 原生渲染器全管线 demo(G5,RFC-0016 §1 管线图;门 G-G5-8)。
//!
//! host 全管线(默认 feature,零 GPU 依赖):
//! geom-build 离线簇化/DAG → GpuScene+MaterialTable+PSO precache → RenderGraph 帧声明
//! (transient + 历史/页表 import + AO/GI 滤波标 AsyncCompute 车道)→ 每帧:流送 tick
//! → 两级剔除 → VisBuffer → classify/resolve → GBuffer → VSM(mark/alloc/raster/sample)
//! → 屏幕探针 GI(时域累积经 temporal 公共底座)→ RTAO+硬阴影(denoise 时域滤波)→
//! 单层材质延迟着色 → TAA → TSR 超分。
//!
//! device 腿(feature `vulkan` + `--device`):经 rurix-rt `render_exec`(RFC-0016 章 B
//! 主通道)跑真多 pass(≥1 raster 真 draw 场景几何 + ≥1 compute 消费 host 光照合成 +
//! readback 像素断言)。效果 kernel 全量 device 化按 RFC-0016 §9.1 R-3 条件臂登记
//! RD-038 存续,本腿为「执行面真派发」的 honest 边界,不伪造效果 device 绿。
//!
//! CLI:`uc06-renderer [--frames N=8] [--size WxH=256x144] [--device] [--dump-graph p] [--json]`
//! `--json` 输出单行 JSON(smoke 脚本消费,字段集冻结);exit 0 仅当全部断言过。

#[cfg(feature = "device-frame")]
mod device_frame;
#[cfg(feature = "vulkan")]
mod device_frame_temporal;
#[cfg(feature = "vulkan")]
mod device_g75;
#[cfg(feature = "vulkan")]
mod device_g75_hw;
#[cfg(feature = "vulkan")]
mod device_kernels;
#[cfg(feature = "vulkan")]
mod device_m19;
#[cfg(feature = "vulkan")]
mod device_m37;
#[cfg(feature = "vulkan")]
mod device_w3;
mod graph_setup;
mod pipeline;
mod scene;
mod shading;
#[cfg(feature = "device-frame")]
mod tiny_sha256;

use rurix_render::gi::probe::GiCamera;
use rurix_render::temporal::common::Mat4;

/// 相机矩阵便捷面(scene 单测与 pipeline 共用)。
pub fn camera_matrices(w: u32, h: u32) -> pipeline::CameraMats {
    pipeline::camera_matrices(w, h)
}

/// GI 场景(每帧同源;材质 albedo 解包自 MaterialTable)。
pub fn gi_scene_of(scene: &scene::Uc06Scene) -> rurix_render::gi::tracer::GiScene {
    pipeline::gi_scene_of(scene)
}

/// 场景 GBuffer(shading::scene_gbuffer 直通)。
pub fn shading_gbuffer(
    scene: &scene::Uc06Scene,
    camera: &GiCamera,
    w: u32,
    h: u32,
    _view_proj: &Mat4,
) -> (
    rurix_render::temporal::image::ImageF32,
    rurix_render::temporal::image::ImageF32,
) {
    shading::scene_gbuffer(scene, camera, w, h)
}

use pipeline::{FrameCtx, PipelineState, RenderConfig, run_frame};

/// G7.4 W3c `--w3-effects` 模式(CI 步骤 94 device 段驱动;三态口径镜像
/// `bin/vk_ray_query`:`W3: PASS` / `W3: SKIP`(dev-env degrade,退 0)/
/// `W3: FAIL`(退 1))。
///
/// `--w3-red-tamper` 走 RED 轴:篡改 device 侧顶点后**必须**对拍失败,失败即 RED-OK。
#[cfg(feature = "vulkan")]
fn run_w3_effects_mode(cli: &Cli) -> i32 {
    let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");
    let scene = scene::build_scene();

    if cli.w3_red_tamper {
        return match device_w3::red_tamper_geometry(&scene) {
            None => {
                println!("W3: SKIP RED-tamper 轴无 device(dev-env degrade)");
                i32::from(require_real)
            }
            Some(true) => {
                println!("W3: RED-OK tamper-geometry(篡改 device 顶点 → 对拍红)");
                0
            }
            Some(false) => {
                eprintln!("W3: FAIL RED-tamper 失效(篡改后对拍仍通过 = 数据流未真实生效)");
                1
            }
        };
    }

    let Some(res) = device_w3::run_w3_effects(&scene) else {
        println!("W3: SKIP 无 Vulkan device / W3 能力链缺失(dev-env degrade)");
        return i32::from(require_real);
    };
    let r = match res {
        Ok(r) => r,
        Err(e) => {
            eprintln!("W3: FAIL 三核 device 执行: {e}");
            return 1;
        }
    };
    println!("{}", r.json());
    // 注入式 RED 轴(过期 TLAS 恒跑;错误 barrier 需 RURIX_VK_VALIDATION=1)。
    let stale = device_w3::red_stale_tlas(&scene);
    match stale {
        Some(true) => println!("W3: RED-OK stale-tlas(过期 TLAS fail-closed)"),
        Some(false) => {
            eprintln!("W3: FAIL RED-stale-tlas 失效(悬垂 TLAS 仍被消费)");
            return 1;
        }
        None => println!("W3: RED-stale-tlas 轴未跑(无 device)"),
    }
    match device_w3::red_wrong_barrier(&scene) {
        Some(true) => println!("W3: RED-OK wrong-barrier(validation 拦截 fail-closed)"),
        Some(false) => {
            eprintln!("W3: FAIL RED-wrong-barrier 失效(非法 barrier 未被 validation 拦截)");
            return 1;
        }
        None => println!("W3: RED-wrong-barrier 轴未跑(RURIX_VK_VALIDATION≠1)"),
    }
    if !r.all_pass() {
        eprintln!("W3: FAIL 三核对拍未全过(见 JSON measured_*/tol_*)");
        return 1;
    }
    println!(
        "W3: PASS shared_tlas={} rays={} pixels={} t={:.3e} bary={:.3e} radiance={:.3e} \
         ao={:.3e} visibility={:.3e}(三核共用同一真实 TLAS + host oracle 对拍全过)",
        r.shared_tlas,
        r.probe_rays,
        r.gbuffer_pixels,
        r.measured_t_max_abs,
        r.measured_bary_max_abs,
        r.measured_radiance_max_abs,
        r.measured_ao_max_abs,
        r.measured_visibility_max_abs,
    );
    0
}

/// G8.4 M37 `--stream-io` 模式(device 必需;`RURIX_REQUIRE_REAL=1`)。
#[cfg(feature = "vulkan")]
fn run_stream_io_mode(cli: &Cli) -> i32 {
    let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");
    let golden = std::path::PathBuf::from(
        cli.golden_dir
            .as_deref()
            .unwrap_or("tests/geom_pages/golden"),
    );
    match device_m37::run_stream_io(&golden) {
        None => {
            println!("M37: SKIP 无 Vulkan device(dev-env degrade)");
            i32::from(require_real)
        }
        Some(Ok(json)) => {
            println!("{json}");
            if json.contains("\"pass\":true") {
                println!("M37: PASS");
                0
            } else {
                eprintln!("M37: FAIL checks 未全绿");
                1
            }
        }
        Some(Err(e)) => {
            eprintln!("M37: FAIL {e}");
            1
        }
    }
}

/// G8.4 门-GeomPage `--geom-page` 模式(device 必需;`RURIX_REQUIRE_REAL=1`)。
#[cfg(feature = "vulkan")]
fn run_geom_page_mode(cli: &Cli) -> i32 {
    let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");
    let golden = std::path::PathBuf::from(
        cli.golden_dir
            .as_deref()
            .unwrap_or("tests/geom_pages/golden"),
    );
    match device_m37::run_geom_page(&golden) {
        None => {
            println!("GeomPage: SKIP 无 Vulkan device(dev-env degrade)");
            i32::from(require_real)
        }
        Some(Ok(json)) => {
            println!("{json}");
            if json.contains("\"pass\":true") {
                println!("GeomPage: PASS");
                0
            } else {
                eprintln!("GeomPage: FAIL checks 未全绿");
                1
            }
        }
        Some(Err(e)) => {
            eprintln!("GeomPage: FAIL {e}");
            1
        }
    }
}

/// G8.5a M19 `--m19-vsm-page-cache` 模式(device 必需;`RURIX_REQUIRE_REAL=1`)。
#[cfg(feature = "vulkan")]
fn run_m19_vsm_page_cache_mode(cli: &Cli) -> i32 {
    let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");
    match device_m19::run_m19_device(cli.m19_red_stale, cli.m19_red_missing_local) {
        None => {
            println!("M19: SKIP 无 Vulkan device(dev-env degrade)");
            i32::from(require_real)
        }
        Some(Ok(json)) => {
            println!("{json}");
            if json.contains("\"pass\":true") || json.contains("\"red_ok\":true") {
                println!("M19: PASS");
                0
            } else {
                eprintln!("M19: FAIL device 对拍未过");
                1
            }
        }
        Some(Err(e)) => {
            eprintln!("M19: FAIL {e}");
            1
        }
    }
}

/// G7.5 `--g75-residuals` 模式(CI 步骤 95 device 段驱动;三态口径镜像
/// `--w3-effects`:`G75: PASS` / `G75: SKIP`(dev-env degrade,退 0)/
/// `G75: FAIL`(退 1))。
///
/// 覆盖 RD-038 余项两轴:VSM 页内深度光栅 + 阴影采样、TSR 空间超分核。
/// HW 光栅轴**不在此模式**——其阻断为编译面(图形 body 最小切片),由步骤 95
/// host 段以 rurixc 真实诊断机验(blocked-honest),不在 device 段伪造条目。
#[cfg(feature = "vulkan")]
fn run_g75_residuals_mode(cli: &Cli) -> i32 {
    let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");
    let scene = scene::build_scene();

    if cli.g75_red_vsm {
        return match device_g75::red_tamper_vsm_depth(&scene) {
            None => {
                println!("G75: SKIP RED-vsm 轴无 device(dev-env degrade)");
                i32::from(require_real)
            }
            Some(true) => {
                println!("G75: RED-OK tamper-vsm-depth(篡改 device 灯空间三角形 → 深度对拍红)");
                0
            }
            Some(false) => {
                eprintln!("G75: FAIL RED-vsm 失效(篡改后深度对拍仍通过 = 数据流未真实生效)");
                1
            }
        };
    }
    if cli.g75_red_tsr {
        return match device_g75::red_tamper_tsr_jitter(&scene) {
            None => {
                println!("G75: SKIP RED-tsr 轴无 device(dev-env degrade)");
                i32::from(require_real)
            }
            Some(true) => {
                println!("G75: RED-OK tamper-tsr-jitter(篡改 device jitter → 重采样对拍红)");
                0
            }
            Some(false) => {
                eprintln!("G75: FAIL RED-tsr 失效(相位错位后对拍仍通过)");
                1
            }
        };
    }

    let Some(res) = device_g75::run_g75_residuals(&scene) else {
        println!("G75: SKIP 无 Vulkan device / W1 能力链缺失(dev-env degrade)");
        return i32::from(require_real);
    };
    let r = match res {
        Ok(r) => r,
        Err(e) => {
            eprintln!("G75: FAIL 余项 device 执行: {e}");
            return 1;
        }
    };
    println!("{}", r.json());
    if !r.all_pass() {
        eprintln!("G75: FAIL 余项对拍未全过(见 JSON measured_*/tol_*)");
        return 1;
    }
    println!(
        "G75: PASS vsm_depth={:.3e}/{} texels(covered={}) vsm_sample={:.3e}/{} samples \
         (shadowed={:.3}) tsr={:.3e}/{} channels(clamped={})",
        r.measured_vsm_depth_max_abs,
        r.vsm_depth_texels,
        r.vsm_depth_covered_texels,
        r.measured_vsm_sample_max_abs,
        r.vsm_samples,
        r.vsm_shadowed_ratio_device,
        r.measured_tsr_max_abs,
        r.tsr_channels,
        r.tsr_clamped_channels,
    );
    0
}

/// G7.5b `--g75-hw-raster` 模式(CI 步骤 95 device 段 HW 腿;三态口径镜像
/// `--g75-residuals`:`G75HW: PASS` / `G75HW: SKIP`(dev-env degrade,退 0)/
/// `G75HW: FAIL`(退 1))。
///
/// 判据 = SW/HW VisBuffer **整数域逐词 diff = 0**(零容差)+ 覆盖非退化
/// (hw==sw covered>0)+ SW 与 host oracle 覆盖集合对齐。保守光栅扩展缺失 → fail-closed
/// **FAIL**(非 SKIP:本机已探明支持〔RFC-0018 §E〕,不启用降级臂)。
/// 两 RED 轴(`--g75-hw-red-tamper-varying` / `--g75-hw-red-tamper-ids`)
/// 篡改仅落 HW 顶点流 → diff 必非零(数据流反证)。
#[cfg(feature = "vulkan")]
fn run_g75_hw_raster_mode(cli: &Cli) -> i32 {
    let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");
    let scene = scene::build_scene();

    type RedAxis = fn(&scene::Uc06Scene) -> Option<Result<u32, String>>;
    let red_axes: [(bool, &str, RedAxis); 2] = [
        (
            cli.g75_hw_red_varying,
            "tamper-varying",
            device_g75_hw::red_tamper_varying,
        ),
        (
            cli.g75_hw_red_ids,
            "tamper-ids",
            device_g75_hw::red_tamper_ids,
        ),
    ];
    for (on, axis, run) in red_axes {
        if !on {
            continue;
        }
        return match run(&scene) {
            None => {
                println!("G75HW: SKIP RED-{axis} 轴无 device(dev-env degrade)");
                i32::from(require_real)
            }
            Some(Err(e)) => {
                eprintln!("G75HW: FAIL RED-{axis} device 执行: {e}");
                1
            }
            Some(Ok(diff)) if diff > 0 => {
                println!("G75HW: RED-OK {axis}(diff_pixels={diff};篡改 HW 顶点流 → 对拍红)");
                0
            }
            Some(Ok(_)) => {
                eprintln!("G75HW: FAIL RED-{axis} 失效(篡改后 diff 仍为 0 = 数据流未真实生效)");
                1
            }
        };
    }

    let Some(res) = device_g75_hw::run_g75_hw_raster(&scene) else {
        println!("G75HW: SKIP 无 Vulkan device / W2 能力链缺失(dev-env degrade)");
        return i32::from(require_real);
    };
    let r = match res {
        Ok(r) => r,
        Err(e) => {
            eprintln!("G75HW: FAIL HW 光栅 device 执行(含保守光栅 fail-closed): {e}");
            return 1;
        }
    };
    println!("{}", r.json());
    if !r.all_pass() {
        eprintln!("G75HW: FAIL SW/HW 整数域对拍未过(见 JSON diff_pixels/covered/oracle)");
        return 1;
    }
    println!(
        "G75HW: PASS diff_pixels={} covered={}/{} words(sw==hw) oracle_bitexact={} \
         triangles={} pipeline={}(保守光栅 OVERESTIMATE + FS 复刻 SW 判定,整数域零容差)",
        r.diff_pixels, r.hw_covered_words, r.pixels, r.oracle_bitexact, r.triangles, r.pipeline,
    );
    0
}

/// G7.6 PR-1 `--g76-tsr-temporal` 模式:TSR 时域臂孤立腿 32 帧对拍
/// (`DeviceFrameSession` + hist ping-pong vs `TsrUpscaler::upscale`)。
#[cfg(feature = "vulkan")]
fn run_g76_tsr_temporal_mode(cli: &Cli) -> i32 {
    let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");

    if cli.g76_red_history {
        return match device_frame_temporal::red_wrong_history() {
            None => {
                println!("G76: SKIP RED-history 轴无 device(dev-env degrade)");
                i32::from(require_real)
            }
            Some(true) => {
                println!("G76: RED-OK wrong-history(不轮换 ping-pong → 时域对拍红)");
                0
            }
            Some(false) => {
                eprintln!("G76: FAIL RED-history 失效(错绑后对拍仍通过 = 双缓冲未真实生效)");
                1
            }
        };
    }

    let Some(res) = device_frame_temporal::run_g76_tsr_temporal() else {
        println!("G76: SKIP 无 Vulkan device / W1 能力链缺失(dev-env degrade)");
        return i32::from(require_real);
    };
    let r = match res {
        Ok(r) => r,
        Err(e) => {
            eprintln!("G76: FAIL TSR 时域臂 device 执行: {e}");
            return 1;
        }
    };
    println!("{}", r.json());
    if !r.history_red_ok {
        eprintln!("G76: FAIL 历史错绑 RED 轴未触发");
        return 1;
    }
    if !r.all_pass() {
        eprintln!("G76: FAIL 时域臂对拍/专项断言未全过(见 JSON)");
        return 1;
    }
    println!(
        "G76: PASS temporal={:.3e} frame0_bitexact={} flicker_mono={} history_red={}",
        r.measured_temporal_max_abs,
        r.frame0_passthrough_bitexact,
        r.flicker_monotone_converged,
        r.history_red_ok,
    );
    0
}

/// G7.6 PR-2 `--device-frame` 模式:15-pass One True Device Frame
/// (`DeviceFrameSession` + `execute_with_frame_update`;禁 `execute_frame`)。
#[cfg(feature = "device-frame")]
fn run_device_frame_mode(cli: &Cli) -> i32 {
    let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");
    let red = if cli.frame_red_visbuffer {
        Some(device_frame::RedAxis::Visbuffer)
    } else if cli.frame_red_history {
        Some(device_frame::RedAxis::History)
    } else if cli.frame_red_jitter {
        Some(device_frame::RedAxis::Jitter)
    } else if cli.frame_red_provenance {
        Some(device_frame::RedAxis::Provenance)
    } else {
        None
    };
    let opts = device_frame::DeviceFrameOptions {
        frames: cli.frames,
        soak: cli.soak,
        min_minutes: cli.min_minutes,
        red,
        json: cli.json,
    };
    let Some(res) = device_frame::run_device_frame(&opts) else {
        println!("FRAME: SKIP 无 Vulkan device / W3 能力链缺失(dev-env degrade)");
        return i32::from(require_real);
    };
    let r = match res {
        Ok(r) => r,
        Err(e) => {
            eprintln!("FRAME: FAIL device frame 执行: {e}");
            return 1;
        }
    };
    println!("{}", r.json());
    if let Some(ok) = r.red_ok {
        if ok {
            println!("FRAME: RED-OK {}", r.red_axis.unwrap_or("?"));
            return 0;
        }
        eprintln!("FRAME: FAIL RED 轴未触发({})", r.red_axis.unwrap_or("?"));
        return 1;
    }
    if let Some(soak) = &r.soak_telemetry {
        if soak.ok {
            println!(
                "FRAME: SOAK-PASS frames={} minutes={:.2} gpu_p95={:.3}ms cpu_p95={:.3}ms vram={:.1}MB",
                soak.actual_frames,
                soak.elapsed_minutes,
                soak.frame_gpu_p95_ms,
                soak.cpu_submit_p95_ms,
                soak.peak_vram_mb,
            );
            return 0;
        }
        eprintln!(
            "FRAME: SOAK-FAIL frames={} minutes={:.2} reasons={:?}",
            soak.actual_frames, soak.elapsed_minutes, soak.fail_reasons
        );
        return 1;
    }
    if !r.all_pass() {
        eprintln!("FRAME: FAIL 帧链对拍/非退化/provenance 未全过(见 JSON)");
        return 1;
    }
    println!(
        "FRAME: PASS frames={} covered={} mv_nz={} xform_changed={} val={} lost={}",
        r.frames,
        r.covered_pixels,
        r.mv_nonzero_count,
        r.instance_transform_changed,
        r.validation_error_count,
        r.device_lost_count,
    );
    0
}

/// CLI 参数(解析确定性;未知参数 = Err)。
#[derive(Debug, Clone)]
struct Cli {
    frames: u32,
    width: u32,
    height: u32,
    device: bool,
    dump_graph: Option<String>,
    json: bool,
    /// G7.4 W3c:三效果核共用同一真实 TLAS 的 device 对拍模式(独立于 `--device`,
    /// 不改 `--device` 既有 JSON 字段集;CI 步骤 94 消费)。
    w3_effects: bool,
    /// `--w3-effects` 的 RED 轴(篡改 device 顶点 → 对拍必红)。
    w3_red_tamper: bool,
    /// G7.5:RD-038 余项(VSM 深度/采样 + TSR)device 对拍模式;CI 步骤 95 消费。
    g75_residuals: bool,
    /// `--g75-residuals` 的 RED 轴:篡改 device 灯空间三角形 → 深度对拍必红。
    g75_red_vsm: bool,
    /// `--g75-residuals` 的 RED 轴:篡改 device jitter → TSR 对拍必红。
    g75_red_tsr: bool,
    /// G7.5b:HW 光栅 SW/HW VisBuffer 整数域 diff=0 对拍模式(真实 graphics
    /// pipeline + 保守光栅 OVERESTIMATE + FS 复刻判定;CI 步骤 95 device 段消费)。
    g75_hw_raster: bool,
    /// `--g75-hw-raster` 的 RED 轴:篡改 winner 三角形 flat varying → diff 必非零。
    g75_hw_red_varying: bool,
    /// `--g75-hw-raster` 的 RED 轴:篡改 winner 三角形 ids 一元 → diff 必非零。
    g75_hw_red_ids: bool,
    /// G7.6 PR-1:TSR 时域臂孤立腿 32 帧对拍(`DeviceFrameSession` + ping-pong)。
    g76_tsr_temporal: bool,
    /// G8.5a M19:VSM 跨帧页缓存 multi-view depth device 对拍。
    m19_vsm_page_cache: bool,
    /// M19 RED:stale z_range 上传。
    m19_red_stale: bool,
    /// M19 RED:local 页不入批。
    m19_red_missing_local: bool,
    /// G8.4 M37:流送 I/O 全链 device 对拍。
    stream_io: bool,
    /// G8.4 门-GeomPage:按需驻留 + LRU + 迟到页。
    geom_page: bool,
    /// M37/GeomPage golden 目录(默认 tests/geom_pages/golden)。
    golden_dir: Option<String>,
    /// `--g76-tsr-temporal` 的 RED 轴:故意不轮换历史绑定 → 时域对拍必红。
    g76_red_history: bool,
    /// G7.6 PR-2:15-pass One True Device Frame。
    device_frame: bool,
    /// soak 取证(帧数与分钟双下界)。
    soak: bool,
    /// soak 最小分钟数。
    min_minutes: f64,
    frame_red_visbuffer: bool,
    frame_red_history: bool,
    frame_red_jitter: bool,
    frame_red_provenance: bool,
}

impl Default for Cli {
    fn default() -> Self {
        Cli {
            frames: 8,
            width: 256,
            height: 144,
            device: false,
            dump_graph: None,
            json: false,
            w3_effects: false,
            w3_red_tamper: false,
            g75_residuals: false,
            g75_red_vsm: false,
            g75_red_tsr: false,
            g75_hw_raster: false,
            g75_hw_red_varying: false,
            g75_hw_red_ids: false,
            g76_tsr_temporal: false,
            g76_red_history: false,
            m19_vsm_page_cache: false,
            m19_red_stale: false,
            m19_red_missing_local: false,
            stream_io: false,
            geom_page: false,
            golden_dir: None,
            device_frame: false,
            soak: false,
            min_minutes: 0.0,
            frame_red_visbuffer: false,
            frame_red_history: false,
            frame_red_jitter: false,
            frame_red_provenance: false,
        }
    }
}

fn parse_cli(args: &[String]) -> Result<Cli, String> {
    let mut c = Cli::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--frames" => {
                i += 1;
                c.frames = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .filter(|&n: &u32| n >= 1)
                    .ok_or("--frames 需要 ≥1 整数")?;
            }
            "--size" => {
                i += 1;
                let s = args.get(i).ok_or("--size 需要 WxH")?;
                let (w, h) = s.split_once('x').ok_or("--size 形如 256x144")?;
                c.width = w.parse().map_err(|_| "--size 宽非整数")?;
                c.height = h.parse().map_err(|_| "--size 高非整数")?;
                if c.width == 0 || c.height == 0 || c.width > 4096 || c.height > 4096 {
                    return Err("--size 越界(1..=4096)".to_owned());
                }
            }
            "--device" => c.device = true,
            "--json" => c.json = true,
            "--w3-effects" => c.w3_effects = true,
            "--w3-red-tamper" => {
                c.w3_effects = true;
                c.w3_red_tamper = true;
            }
            "--g75-residuals" => c.g75_residuals = true,
            "--g75-red-vsm" => {
                c.g75_residuals = true;
                c.g75_red_vsm = true;
            }
            "--g75-red-tsr" => {
                c.g75_residuals = true;
                c.g75_red_tsr = true;
            }
            "--g75-hw-raster" => c.g75_hw_raster = true,
            "--g75-hw-red-tamper-varying" => {
                c.g75_hw_raster = true;
                c.g75_hw_red_varying = true;
            }
            "--g75-hw-red-tamper-ids" => {
                c.g75_hw_raster = true;
                c.g75_hw_red_ids = true;
            }
            "--g76-tsr-temporal" => c.g76_tsr_temporal = true,
            "--g76-red-history" => {
                c.g76_tsr_temporal = true;
                c.g76_red_history = true;
            }
            "--m19-vsm-page-cache" => c.m19_vsm_page_cache = true,
            "--m19-red-stale" => {
                c.m19_vsm_page_cache = true;
                c.m19_red_stale = true;
            }
            "--m19-red-missing-local" => {
                c.m19_vsm_page_cache = true;
                c.m19_red_missing_local = true;
            }
            "--stream-io" => c.stream_io = true,
            "--geom-page" => c.geom_page = true,
            "--golden-dir" => {
                i += 1;
                c.golden_dir = Some(args.get(i).ok_or("--golden-dir 需要路径")?.clone());
            }
            "--device-frame" => c.device_frame = true,
            "--soak" => c.soak = true,
            "--min-minutes" => {
                i += 1;
                c.min_minutes = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .filter(|&n: &f64| n >= 0.0)
                    .ok_or("--min-minutes 需要 ≥0 浮点")?;
            }
            "--frame-red-visbuffer" => {
                c.device_frame = true;
                c.frame_red_visbuffer = true;
            }
            "--frame-red-history" => {
                c.device_frame = true;
                c.frame_red_history = true;
            }
            "--frame-red-jitter" => {
                c.device_frame = true;
                c.frame_red_jitter = true;
            }
            "--frame-red-provenance" => {
                c.device_frame = true;
                c.frame_red_provenance = true;
            }
            "--dump-graph" => {
                i += 1;
                c.dump_graph = Some(args.get(i).ok_or("--dump-graph 需要路径")?.clone());
            }
            other => return Err(format!("未知参数 {other}")),
        }
        i += 1;
    }
    Ok(c)
}

/// 主流程(断言失败 → Err 串,exit 非零)。
fn run(cli: &Cli) -> Result<pipeline::Uc06Summary, String> {
    let cfg = RenderConfig {
        out_w: cli.width,
        out_h: cli.height,
        frames: cli.frames,
        ..Default::default()
    };

    let scene = scene::build_scene();
    let mut st = PipelineState::new(&scene, &cfg);
    let mut ctx = FrameCtx::new(&scene);

    let mut summaries = Vec::with_capacity(cli.frames as usize);
    for frame in 0..cli.frames {
        let s = run_frame(&scene, &mut st, &mut ctx, &cfg, frame);
        summaries.push(s);
    }

    // 图 dump(可选;每帧同构,末帧足够)。
    if let Some(path) = &cli.dump_graph {
        let dump = st.compiled.dump_json();
        std::fs::write(path, dump).map_err(|e| format!("写 dump-graph 失败: {e}"))?;
    }

    let mut summary = pipeline::assemble_summary(&scene, &st, &summaries, cli.device)?;
    summary.width = cli.width;
    summary.height = cli.height;
    summary.internal_width = cfg.internal_w();
    summary.internal_height = cfg.internal_h();
    Ok(summary)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match parse_cli(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("uc06-renderer: {e}");
            std::process::exit(2);
        }
    };

    #[cfg(not(feature = "vulkan"))]
    if cli.device
        || cli.w3_effects
        || cli.g75_residuals
        || cli.g75_hw_raster
        || cli.g76_tsr_temporal
        || cli.m19_vsm_page_cache
        || cli.stream_io
        || cli.geom_page
    {
        eprintln!(
            "uc06-renderer: --device/--w3-effects/--g75-residuals/--g75-hw-raster/--g76-tsr-temporal/--m19-vsm-page-cache/--stream-io/--geom-page 需要 feature vulkan"
        );
        std::process::exit(2);
    }

    #[cfg(not(feature = "device-frame"))]
    if cli.device_frame
        || cli.soak
        || cli.frame_red_visbuffer
        || cli.frame_red_history
        || cli.frame_red_jitter
        || cli.frame_red_provenance
    {
        eprintln!(
            "uc06-renderer: --device-frame/--soak/--frame-red-* 需要 feature device-frame(cargo run -p uc06-renderer --features device-frame)"
        );
        std::process::exit(2);
    }

    // G7.4 W3c 模式:三效果核共用同一真实 TLAS device 对拍(独立通道;不跑 host 全管线,
    // 不产 `--device` JSON,故 `--device` 既有字段集 0-byte)。
    #[cfg(feature = "vulkan")]
    if cli.w3_effects {
        std::process::exit(run_w3_effects_mode(&cli));
    }

    // G7.5 余项模式:同为独立通道,`--device` 既有 JSON 字段集 0-byte。
    #[cfg(feature = "vulkan")]
    if cli.g75_residuals {
        std::process::exit(run_g75_residuals_mode(&cli));
    }

    // G7.5b HW 光栅 diff=0 模式:同为独立通道,既有模式字段集 0-byte。
    #[cfg(feature = "vulkan")]
    if cli.g75_hw_raster {
        std::process::exit(run_g75_hw_raster_mode(&cli));
    }

    #[cfg(feature = "vulkan")]
    if cli.g76_tsr_temporal {
        std::process::exit(run_g76_tsr_temporal_mode(&cli));
    }

    #[cfg(feature = "vulkan")]
    if cli.m19_vsm_page_cache {
        std::process::exit(run_m19_vsm_page_cache_mode(&cli));
    }

    #[cfg(feature = "vulkan")]
    if cli.stream_io {
        std::process::exit(run_stream_io_mode(&cli));
    }

    #[cfg(feature = "vulkan")]
    if cli.geom_page {
        std::process::exit(run_geom_page_mode(&cli));
    }

    #[cfg(feature = "device-frame")]
    if cli.device_frame {
        std::process::exit(run_device_frame_mode(&cli));
    }

    match run(&cli) {
        Ok(summary) => {
            // device 腿(feature + flag 双开)。
            #[cfg(feature = "vulkan")]
            let device_json = if cli.device {
                match pipeline::run_device_leg(&summary) {
                    Ok(j) => Some(j),
                    Err(e) => {
                        let require_real =
                            std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");
                        let environment_missing = e.contains("Vulkan loader")
                            || e.contains("no-vulkan")
                            || e.contains("vulkan loader");
                        if require_real || !environment_missing {
                            eprintln!(
                                "uc06-renderer: device 腿失败(回归硬红;仅 loader 缺失可降级): {e}"
                            );
                            std::process::exit(1);
                        }
                        eprintln!("uc06-renderer: device 腿降级(dev-env degrade,不充绿): {e}");
                        None
                    }
                }
            } else {
                None
            };
            #[cfg(not(feature = "vulkan"))]
            let device_json: Option<pipeline::DeviceLeg> = None;

            let json = pipeline::summary_json(&summary, device_json.as_ref(), cli.device);
            if cli.json {
                println!("{json}");
            } else {
                println!("uc06-renderer OK: {}", summary.one_line());
                println!("{json}");
            }
            if !summary.all_asserts_pass(device_json.as_ref()) {
                eprintln!("uc06-renderer: 断言未全过(见 JSON asserts)");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("uc06-renderer: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parse_defaults_and_overrides() {
        let c = parse_cli(&[]).unwrap();
        assert_eq!((c.frames, c.width, c.height), (8, 256, 144));
        assert!(!c.device && !c.json && c.dump_graph.is_none());
        let c = parse_cli(&[
            "--frames".into(),
            "4".into(),
            "--size".into(),
            "128x72".into(),
            "--json".into(),
        ])
        .unwrap();
        assert_eq!((c.frames, c.width, c.height), (4, 128, 72));
        assert!(c.json);
        let c = parse_cli(&["--device".into(), "--dump-graph".into(), "g.json".into()]).unwrap();
        assert!(c.device && c.dump_graph.as_deref() == Some("g.json"));
    }

    #[test]
    fn cli_parse_rejects_bad_input() {
        assert!(parse_cli(&["--frames".into(), "0".into()]).is_err());
        assert!(parse_cli(&["--size".into(), "0x10".into()]).is_err());
        assert!(parse_cli(&["--size".into(), "10".into()]).is_err());
        assert!(parse_cli(&["--bogus".into()]).is_err());
        assert!(parse_cli(&["--frames".into()]).is_err());
    }
}
