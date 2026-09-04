//! G40 体积云实时窗口呈现 harness(门 `g40.clouds.present`)。
//!
//! 复现 HanPi Volume Cloud(<https://github.com/AshenOneArt/HPVolumeCloud>,
//! MIT + 署名要求;其自身派生自 Unity HDRP 体积云)的体积云方案,并接进真窗口
//! 实时车道:程序化物理天空 + 体积云 ray march,飞行相机自由观察。
//!
//! ## 车道(两 pass 持久 session + swapchain 真窗口 present)
//!
//! ```text
//! g40_volumetric_cloud  (逐像素 slab ray march + 锥形光步 + phi_fwd)
//!   → out_color(3 f32/px scene-linear HDR,驻留 device)
//! g40_cloud_encode      (曝光 → ACES filmic → sRGB → BGRA8 打包)
//!   → out_bgra(1 u32/px)→ 回读 8.3MB @1080p → ExternalImagePresent
//! ```
//!
//! device 侧编码是刻意的:回读量从 w×h×12B 降到 w×h×4B(1080p 下 24.9MB →
//! 8.3MB),present 腿不成为带宽瓶颈(G31 A1 教训同律)。
//!
//! ## 与冻结面的关系(0-byte 纪律)
//!
//! 本 bin 与 `g31_window_present` / `g14_3_lane_body` / `display/` 零共享——不
//! include 共享体、不复用生产 kernel、不触碰任何冻结 digest 锚。G40 自带的两个
//! kernel(`kernels/g40_volumetric_cloud.rx` / `kernels/g40_cloud_encode.rx`)
//! 与 host 金标准 `world::clouds` / `world::sky` 公式面逐字同源。
//!
//! ## 天空场景
//!
//! 四档命名预设的太阳高度角/方位角/浊度标定自 Poly Haven CC0「Pure Sky」实拍
//! 天空(逐档出处见 `world::sky` 各预设常量文档);**只取标定数值,不入库任何
//! 二进制资产**。
//!
//! ## 三态纪律
//!
//! 无 Vulkan 设备 / 缺 SPV ⇒ 打印 `skipped_dev_env` 退 0;`RURIX_REQUIRE_REAL=1`
//! 下翻硬红退 1。`--headless` 跳过窗口只跑渲染(CI 面)。
//!
//! ## 用法
//!
//! ```text
//! g40_cloud_present [--preset noon|clear|golden|sunset]
//!     [--width 1280] [--height 720] [--frames N] [--fov 60]
//!     [--phi-fwd on|off] [--coverage F] [--ev F] [--seed N]
//!     [--spv-cloud <a.spv>] [--spv-encode <b.spv>]
//!     [--headless] [--dump <out.png>] [--digest]
//! ```
//!
//! 交互(窗口臂):`WASD` 平移 / `QE` 升降 / 方向键转视角 / 鼠标拖动转视角 /
//! `-` `=` 曝光 ±0.25 EV / `ESC` 退出。

#![forbid(unsafe_code)]

use image_io::{ImageBuffer, ImageFormat, Rgb, encode as encode_image};
use rurix_render::world::clouds::{
    CLOUD_PARAM_COUNT, CloudCamera, CloudParams, NoiseVolumes, pack_cloud_params, pack_weather,
};
use rurix_render::world::sky::{SKY_LUT_H, SKY_LUT_W, Sky, bake_sky_view_lut, preset_by_name};
use rurix_rt::render_exec::{
    Bindings, BufferDesc, BufferUsage, ComputePass, DeviceFrameSession, DispatchSpec, FrameUpdate,
    Pass, Readback, ResourceDesc, StableResourceId, TargetState,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

const TAG: &str = "G40_CLOUDS";

// ── 资源下标(声明序 = 绑定序;与两 kernel 的形参序逐字对应)──────────────
const R_PARAMS: u32 = 0;
const R_WEATHER: u32 = 1;
const R_NOISE_BASE: u32 = 2;
const R_NOISE_DETAIL: u32 = 3;
const R_SKY_LUT: u32 = 4;
const R_COLOR: u32 = 5;
const R_ENC_PARAMS: u32 = 6;
const R_BGRA: u32 = 7;
const R_COUNT: usize = 8;

/// 云 pass 屏障计划(保守超集:触达资源全声明为 storage 读写)。
const PLAN_CLOUD: &[(u32, TargetState)] = &[
    (R_PARAMS, TargetState::StorageReadWrite),
    (R_WEATHER, TargetState::StorageReadWrite),
    (R_NOISE_BASE, TargetState::StorageReadWrite),
    (R_NOISE_DETAIL, TargetState::StorageReadWrite),
    (R_SKY_LUT, TargetState::StorageReadWrite),
    (R_COLOR, TargetState::StorageReadWrite),
];
/// 编码 pass 屏障计划。
const PLAN_ENCODE: &[(u32, TargetState)] = &[
    (R_COLOR, TargetState::StorageReadWrite),
    (R_ENC_PARAMS, TargetState::StorageReadWrite),
    (R_BGRA, TargetState::StorageReadWrite),
];

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

/// 三态:缺设备/缺件 ⇒ 退 0 登记;`RURIX_REQUIRE_REAL=1` ⇒ 翻硬红。
fn skip_or_fail(why: &str) -> ! {
    if std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1") {
        fail(&format!("{why}(RURIX_REQUIRE_REAL=1 下不可跳过)"));
    }
    println!("{TAG}: skipped_dev_env {why}");
    std::process::exit(0)
}

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

struct Args {
    preset: String,
    width: u32,
    height: u32,
    frames: Option<u32>,
    fov_deg: f32,
    phi_fwd: bool,
    coverage: f32,
    ev: Option<f32>,
    seed: u64,
    primary_steps: Option<u32>,
    light_steps: Option<u32>,
    phi_intensity: Option<f32>,
    spv_cloud: PathBuf,
    spv_encode: PathBuf,
    headless: bool,
    dump: Option<PathBuf>,
    digest: bool,
}

fn parse_args() -> Args {
    let root = workspace_root();
    let mut a = Args {
        preset: "clear".to_owned(),
        width: 1280,
        height: 720,
        frames: None,
        fov_deg: 60.0,
        phi_fwd: true,
        coverage: 0.62,
        ev: None,
        seed: 0x5eed_1234,
        primary_steps: None,
        light_steps: None,
        phi_intensity: None,
        spv_cloud: root.join(".tmp/g40/spv/g40_volumetric_cloud.spv"),
        spv_encode: root.join(".tmp/g40/spv/g40_cloud_encode.spv"),
        headless: false,
        dump: None,
        digest: false,
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < argv.len() {
        let need = |i: usize| -> String {
            argv.get(i + 1)
                .cloned()
                .unwrap_or_else(|| fail(&format!("{} 缺参数", argv[i])))
        };
        match argv[i].as_str() {
            "--preset" => {
                a.preset = need(i);
                i += 1;
            }
            "--width" => {
                a.width = need(i).parse().unwrap_or_else(|_| fail("--width 非法"));
                i += 1;
            }
            "--height" => {
                a.height = need(i).parse().unwrap_or_else(|_| fail("--height 非法"));
                i += 1;
            }
            "--frames" => {
                a.frames = Some(need(i).parse().unwrap_or_else(|_| fail("--frames 非法")));
                i += 1;
            }
            "--fov" => {
                a.fov_deg = need(i).parse().unwrap_or_else(|_| fail("--fov 非法"));
                i += 1;
            }
            "--phi-fwd" => {
                a.phi_fwd = match need(i).as_str() {
                    "on" => true,
                    "off" => false,
                    s => fail(&format!("--phi-fwd 闭集 on|off,得到 `{s}`")),
                };
                i += 1;
            }
            "--coverage" => {
                a.coverage = need(i).parse().unwrap_or_else(|_| fail("--coverage 非法"));
                i += 1;
            }
            "--ev" => {
                a.ev = Some(need(i).parse().unwrap_or_else(|_| fail("--ev 非法")));
                i += 1;
            }
            "--seed" => {
                a.seed = need(i).parse().unwrap_or_else(|_| fail("--seed 非法"));
                i += 1;
            }
            "--primary-steps" => {
                a.primary_steps =
                    Some(need(i).parse().unwrap_or_else(|_| fail("--primary-steps 非法")));
                i += 1;
            }
            "--light-steps" => {
                a.light_steps =
                    Some(need(i).parse().unwrap_or_else(|_| fail("--light-steps 非法")));
                i += 1;
            }
            "--phi-intensity" => {
                a.phi_intensity =
                    Some(need(i).parse().unwrap_or_else(|_| fail("--phi-intensity 非法")));
                i += 1;
            }
            "--spv-cloud" => {
                a.spv_cloud = PathBuf::from(need(i));
                i += 1;
            }
            "--spv-encode" => {
                a.spv_encode = PathBuf::from(need(i));
                i += 1;
            }
            "--dump" => {
                a.dump = Some(PathBuf::from(need(i)));
                i += 1;
            }
            "--headless" => a.headless = true,
            "--digest" => a.digest = true,
            s => fail(&format!("未知参数 `{s}`")),
        }
        i += 1;
    }
    if a.width == 0 || a.height == 0 {
        fail("分辨率不可为 0");
    }
    a
}

/// 程序化 weather map(确定性 LCG;coverage 场为多尺度团块,type 通道给出云型分布)。
///
/// 与 `atmosphere::canonical_weather_map` 的纯 LCG 白噪声不同:体积云要求 coverage
/// 是**空间相关**的场(否则云被打成均匀糊状),故这里做三倍频 value-noise 叠加。
fn build_weather_map(seed: u64, coverage: f32) -> rurix_render::world::atmosphere::WeatherMap {
    const N: u32 = 128;
    let hash = |x: i64, y: i64, s: u64| -> f32 {
        let mut h = (x as u64)
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (y as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f)
            ^ s;
        h ^= h >> 29;
        h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        h ^= h >> 32;
        (h >> 40) as f32 / 16_777_216.0
    };
    // 环绕 value noise(周期 = period,保证 map 自身可无缝平铺)。
    let vnoise = |u: f32, v: f32, period: i64, s: u64| -> f32 {
        let fx = u * period as f32;
        let fy = v * period as f32;
        let x0 = fx.floor();
        let y0 = fy.floor();
        let tx = fx - x0;
        let ty = fy - y0;
        let sm = |t: f32| t * t * (3.0 - 2.0 * t);
        let (tx, ty) = (sm(tx), sm(ty));
        let wrap = |i: f32| -> i64 { ((i as i64 % period) + period) % period };
        let (ix0, iy0) = (wrap(x0), wrap(y0));
        let (ix1, iy1) = (wrap(x0 + 1.0), wrap(y0 + 1.0));
        let c00 = hash(ix0, iy0, s);
        let c10 = hash(ix1, iy0, s);
        let c01 = hash(ix0, iy1, s);
        let c11 = hash(ix1, iy1, s);
        let a = c00 + (c10 - c00) * tx;
        let b = c01 + (c11 - c01) * tx;
        a + (b - a) * ty
    };
    let mut pixels = Vec::with_capacity((N * N) as usize);
    for y in 0..N {
        for x in 0..N {
            let u = x as f32 / N as f32;
            let v = y as f32 / N as f32;
            // 三倍频团块(4/8/16 周期);整体偏移到目标覆盖度。
            let f = vnoise(u, v, 4, seed) * 0.55
                + vnoise(u, v, 8, seed ^ 0x11) * 0.30
                + vnoise(u, v, 16, seed ^ 0x22) * 0.15;
            let cov = ((f - 0.5) * 1.9 + coverage).clamp(0.0, 1.0);
            let humidity = vnoise(u, v, 6, seed ^ 0x33).clamp(0.0, 1.0);
            // 云型:覆盖度高处更易长成浓积云/积雨云(物理直觉:湿厚处发展旺盛)。
            let ctype = (cov * 0.7 + vnoise(u, v, 3, seed ^ 0x44) * 0.5).clamp(0.0, 1.0);
            pixels.push([cov, humidity, ctype]);
        }
    }
    rurix_render::world::atmosphere::WeatherMap {
        width: N,
        height: N,
        pixels,
    }
}

fn read_spv(path: &Path) -> Vec<u8> {
    match std::fs::read(path) {
        Ok(b) if b.len() >= 4 && b.len() % 4 == 0 && b[0..4] == [0x03, 0x02, 0x23, 0x07] => b,
        Ok(_) => fail(&format!("SPV 非法(magic/对齐): {}", path.display())),
        Err(e) => skip_or_fail(&format!(
            "SPV 不在位 {}: {e}(先跑 `cargo run -p rurixc --features vulkan-backend --bin rurixc -- \
             src/rurix-render/kernels/g40_volumetric_cloud.rx --target vulkan -o <out.spv>`)",
            path.display()
        )),
    }
}

fn f32s_to_bytes(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

const STORAGE: BufferUsage = BufferUsage {
    storage: true,
    uniform: false,
    vertex: false,
    indirect: false,
};

/// 逐帧上传目标 buffer(须 host-visible —— `FrameUpdate::buffer_uploads` 的
/// 目标不可为 DEVICE_LOCAL,校验期 fail-closed)。
fn upload_buf(bytes: &[u8]) -> ResourceDesc<'_> {
    ResourceDesc::Buffer(BufferDesc {
        size: bytes.len() as u64,
        usage: STORAGE,
        data: Some(bytes),
        device_local: false,
    })
}

/// 创建期一次上传、之后只读的大表(噪声体 / weather / sky LUT)走 DEVICE_LOCAL。
fn static_buf(bytes: &[u8]) -> ResourceDesc<'_> {
    ResourceDesc::Buffer(BufferDesc {
        size: bytes.len() as u64,
        usage: STORAGE,
        data: Some(bytes),
        device_local: true,
    })
}

/// 全 GPU 驻留的中间/输出缓冲(无初始数据)。
fn scratch_buf<'a>(size: u64) -> ResourceDesc<'a> {
    ResourceDesc::Buffer(BufferDesc {
        size,
        usage: STORAGE,
        data: None,
        device_local: true,
    })
}

/// 飞行相机状态(yaw/pitch 度制;世界 Y-up)。
struct FlyCamera {
    pos: [f32; 3],
    yaw_deg: f32,
    pitch_deg: f32,
}

impl FlyCamera {
    fn basis(&self, aspect: f32, tan_half: f32) -> CloudCamera {
        let y = self.yaw_deg.to_radians();
        let p = self.pitch_deg.clamp(-89.0, 89.0).to_radians();
        let forward = [p.cos() * y.sin(), p.sin(), p.cos() * y.cos()];
        // right = normalize(cross(worldUp, forward)),worldUp = +Y ⇒ 水平面内。
        let right = {
            let r = [forward[2], 0.0, -forward[0]];
            let l = (r[0] * r[0] + r[2] * r[2]).sqrt().max(1e-6);
            [r[0] / l, 0.0, r[2] / l]
        };
        // up = cross(forward, right)。注意**不是** cross(right, forward)——后者
        // 得到 −up(其 y 分量恒为 −(fx²+fz²) < 0),画面上下颠倒。
        let up = [
            forward[1] * right[2] - forward[2] * right[1],
            forward[2] * right[0] - forward[0] * right[2],
            forward[0] * right[1] - forward[1] * right[0],
        ];
        CloudCamera {
            origin: self.pos,
            forward,
            right,
            up,
            tan_half_fov: tan_half,
            aspect,
        }
    }
}

fn main() {
    let args = parse_args();
    let preset = preset_by_name(&args.preset).unwrap_or_else(|| {
        fail(&format!(
            "--preset 闭集 noon|clear|golden|sunset,得到 `{}`",
            args.preset
        ))
    });

    // ── host 事实源建造(天空 / 噪声体 / weather map;一次性)──────────────
    let t_bake = Instant::now();
    let sky = Sky::new(preset);
    let noise = NoiseVolumes::bake();
    let weather = build_weather_map(args.seed, args.coverage);
    let sky_lut = bake_sky_view_lut(&sky);
    let bake_ms = t_bake.elapsed().as_secs_f64() * 1000.0;

    let mut params = CloudParams::default();
    if !args.phi_fwd {
        params = params.without_phi_fwd();
    }
    if let Some(n) = args.primary_steps {
        params.primary_steps = n.max(1);
    }
    if let Some(n) = args.light_steps {
        params.light_steps = n.max(1);
    }
    // `--phi-fwd off` 的关臂语义优先:显式关闭时不被强度旋钮复活。
    if let Some(v) = args.phi_intensity {
        if args.phi_fwd {
            params.phi_fwd_intensity = v.max(0.0);
        }
    }
    let exposure = 2.0f32.powf(args.ev.unwrap_or(0.0)) * 8.0;

    // ── SPV 装载 ──────────────────────────────────────────────────────────
    let spv_cloud = read_spv(&args.spv_cloud);
    let spv_encode = read_spv(&args.spv_encode);

    let (w, h) = (args.width, args.height);
    let px_count = (w as u64) * (h as u64);
    let color_bytes = px_count * 12;
    let bgra_bytes = px_count * 4;

    // ── 静态上传字节(创建期一次)──────────────────────────────────────────
    let weather_bytes = f32s_to_bytes(&pack_weather(&weather));
    let noise_base_bytes = f32s_to_bytes(&noise.base);
    let noise_detail_bytes = f32s_to_bytes(&noise.detail);
    let sky_lut_bytes = f32s_to_bytes(&sky_lut);

    let mut cam = FlyCamera {
        // 地表以上 400m,朝太阳所在方位偏一点,便于看到受光面与银边。
        pos: [0.0, 400.0, 0.0],
        yaw_deg: preset.sun_azimuth_deg - 25.0,
        pitch_deg: 12.0,
    };
    let tan_half = (args.fov_deg.to_radians() * 0.5).tan();
    let cam0 = cam.basis(w as f32 / h as f32, tan_half);
    let params_bytes = f32s_to_bytes(&pack_cloud_params(
        &params, &sky, &cam0, w, h, &weather, &noise, SKY_LUT_W, SKY_LUT_H,
    ));
    let enc_params_bytes = f32s_to_bytes(&[w as f32, h as f32, exposure, 1.0 / 2.2]);

    let resources = vec![
        upload_buf(&params_bytes),          // R_PARAMS(逐帧改相机)
        static_buf(&weather_bytes),         // R_WEATHER
        static_buf(&noise_base_bytes),      // R_NOISE_BASE
        static_buf(&noise_detail_bytes),    // R_NOISE_DETAIL
        static_buf(&sky_lut_bytes),         // R_SKY_LUT
        scratch_buf(color_bytes),           // R_COLOR
        upload_buf(&enc_params_bytes),      // R_ENC_PARAMS(逐帧改曝光)
        scratch_buf(bgra_bytes),            // R_BGRA
    ];
    assert_eq!(resources.len(), R_COUNT, "资源表长度须与下标闭集一致");

    let groups = [w.div_ceil(8), h.div_ceil(8), 1];
    let passes = vec![
        Pass::Compute(ComputePass {
            name: "g40_volumetric_cloud",
            spirv: &spv_cloud,
            entry: None,
            dispatch: DispatchSpec::Direct(groups),
            bindings: Bindings {
                storage_buffers: vec![
                    R_PARAMS,
                    R_WEATHER,
                    R_NOISE_BASE,
                    R_NOISE_DETAIL,
                    R_SKY_LUT,
                    R_COLOR,
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g40_cloud_encode",
            spirv: &spv_encode,
            entry: None,
            dispatch: DispatchSpec::Direct(groups),
            bindings: Bindings {
                storage_buffers: vec![R_COLOR, R_ENC_PARAMS, R_BGRA],
                ..Bindings::default()
            },
        }),
    ];
    let barriers: Vec<&[(u32, TargetState)]> = vec![PLAN_CLOUD, PLAN_ENCODE];
    let readbacks = vec![Readback::Buffer {
        res: R_BGRA,
        offset: 0,
        size: bgra_bytes,
    }];

    let mut session = match DeviceFrameSession::new(&resources, &passes, &barriers, &readbacks, 2) {
        Ok(s) => s,
        Err(e) => skip_or_fail(&format!("device session 建立失败: {e}")),
    };

    // ── 窗口(headless 臂跳过)────────────────────────────────────────────
    let mut window = if args.headless {
        None
    } else {
        match rurix_rt::vk::ExternalImagePresent::create(w, h, "Rurix G40 — 体积云", true) {
            Ok(win) => Some(win),
            Err(e) => {
                if std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1") {
                    fail(&format!("窗口建立失败: {e}"));
                }
                eprintln!("{TAG}: 窗口建立失败({e}),退化为 headless");
                None
            }
        }
    };

    println!(
        "{TAG}: start preset={} sun_elev={:.2}deg ref={} {}x{} phi_fwd={} bake_ms={bake_ms:.1}",
        preset.name,
        preset.sun_elevation_deg,
        preset.reference_slug,
        w,
        h,
        if args.phi_fwd { "on" } else { "off" },
    );

    // ── 帧循环 ────────────────────────────────────────────────────────────
    let max_frames = args.frames.unwrap_or(if window.is_some() { u32::MAX } else { 1 });
    let mut frame = 0u32;
    let mut ev_bias = args.ev.unwrap_or(0.0);
    let mut last_bgra: Vec<u8> = Vec::new();
    let mut render_ms_sum = 0.0f64;
    let mut render_ms_count = 0u32;
    let t_loop = Instant::now();

    while frame < max_frames {
        // 输入 → 相机/曝光。
        if let Some(win) = window.as_mut() {
            let input = win.poll_input();
            if input.close_requested {
                break;
            }
            if input.minimized {
                continue;
            }
            // WASD 平移 / QE 升降(米/帧);方向键 + 鼠标转视角。
            let speed = 60.0f32;
            let y = cam.yaw_deg.to_radians();
            let (fx, fz) = (y.sin(), y.cos());
            if input.key(0x57) {
                cam.pos[0] += fx * speed;
                cam.pos[2] += fz * speed;
            }
            if input.key(0x53) {
                cam.pos[0] -= fx * speed;
                cam.pos[2] -= fz * speed;
            }
            if input.key(0x41) {
                cam.pos[0] -= fz * speed;
                cam.pos[2] += fx * speed;
            }
            if input.key(0x44) {
                cam.pos[0] += fz * speed;
                cam.pos[2] -= fx * speed;
            }
            if input.key(0x45) {
                cam.pos[1] += speed;
            }
            if input.key(0x51) {
                cam.pos[1] = (cam.pos[1] - speed).max(10.0);
            }
            if input.key(0x25) {
                cam.yaw_deg -= 2.0;
            }
            if input.key(0x27) {
                cam.yaw_deg += 2.0;
            }
            if input.key(0x26) {
                cam.pitch_deg += 1.5;
            }
            if input.key(0x28) {
                cam.pitch_deg -= 1.5;
            }
            cam.yaw_deg += input.mouse_dx as f32 * 0.15;
            cam.pitch_deg = (cam.pitch_deg - input.mouse_dy as f32 * 0.15).clamp(-89.0, 89.0);
            if input.key(0xBD) {
                ev_bias -= 0.25;
            }
            if input.key(0xBB) {
                ev_bias += 0.25;
            }
        }

        // 逐帧参数上传(相机 + 曝光;其余大表创建期驻留不动)。
        let camf = cam.basis(w as f32 / h as f32, tan_half);
        let p = pack_cloud_params(
            &params, &sky, &camf, w, h, &weather, &noise, SKY_LUT_W, SKY_LUT_H,
        );
        debug_assert_eq!(p.len(), CLOUD_PARAM_COUNT);
        let exp = 2.0f32.powf(ev_bias) * 8.0;
        let update = FrameUpdate {
            // StableResourceId 为 1-based(资源表下标 + 1)。
            buffer_uploads: vec![
                (
                    StableResourceId(u64::from(R_PARAMS) + 1),
                    0,
                    f32s_to_bytes(&p),
                ),
                (
                    StableResourceId(u64::from(R_ENC_PARAMS) + 1),
                    0,
                    f32s_to_bytes(&[w as f32, h as f32, exp, 1.0 / 2.2]),
                ),
            ],
            // `execute_with_frame_update` 路径下 `None` = **不回读**(与 `execute()`
            // 的全量语义相反),present 必须显式点名 BGRA 回读。
            readback_subset: Some(vec![0]),
            ..FrameUpdate::default()
        };

        let t_frame = Instant::now();
        // provenance 由执行器按本帧 update 预推,再原样回交 —— 生产提交路径同律
        // (stale allocation/generation 在 vkQueueSubmit 前确定性拒)。
        let expected = match session.next_provenance_with_update(&update) {
            Ok(p) => p,
            Err(e) => fail(&format!("帧 {frame} provenance 预推失败: {e}")),
        };
        let out = match session.execute_with_frame_update(&expected, &update) {
            Ok(o) => o,
            Err(e) => fail(&format!("帧 {frame} 提交失败: {e}")),
        };
        let ms = t_frame.elapsed().as_secs_f64() * 1000.0;
        if frame > 0 {
            render_ms_sum += ms;
            render_ms_count += 1;
        }
        last_bgra = out.readbacks.into_iter().next().unwrap_or_default();

        if let Some(win) = window.as_mut() {
            if let Err(e) = win.present_rgba8(&last_bgra) {
                fail(&format!("帧 {frame} present 失败: {e}"));
            }
        }
        frame += 1;
    }

    let wall_s = t_loop.elapsed().as_secs_f64();
    let mean_ms = if render_ms_count > 0 {
        render_ms_sum / f64::from(render_ms_count)
    } else {
        0.0
    };
    let fps = if mean_ms > 0.0 { 1000.0 / mean_ms } else { 0.0 };

    // ── 出图 / digest ─────────────────────────────────────────────────────
    if let Some(path) = args.dump.as_ref() {
        if last_bgra.len() as u64 != bgra_bytes {
            fail("回读长度与 BGRA8 期望不符");
        }
        let mut buf = ImageBuffer::new(w, h, Rgb::new(0.0, 0.0, 0.0));
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                // 回读为 BGRA8;image-io 取 [0,1] 线性分量,此处已是显示编码值。
                let b = f32::from(last_bgra[i]) / 255.0;
                let g = f32::from(last_bgra[i + 1]) / 255.0;
                let r = f32::from(last_bgra[i + 2]) / 255.0;
                buf.set(x, y, Rgb::new(r, g, b));
            }
        }
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let bytes = encode_image(&buf, ImageFormat::Png)
            .unwrap_or_else(|e| fail(&format!("PNG 编码失败: {e}")));
        std::fs::write(path, &bytes)
            .unwrap_or_else(|e| fail(&format!("写 {} 失败: {e}", path.display())));
        println!("{TAG}: dump {}", path.display());
    }

    let digest = if args.digest {
        let d = rurix_pkg::sha256::digest(&last_bgra);
        format!(
            " digest=sha256:{}",
            d.iter().map(|b| format!("{b:02x}")).collect::<String>()
        )
    } else {
        String::new()
    };

    println!(
        "{TAG}: PASS preset={} frames={frame} render_frame_ms={mean_ms:.3} fps={fps:.1} \
         wall_s={wall_s:.2} phi_fwd={}{digest}",
        preset.name,
        if args.phi_fwd { "on" } else { "off" },
    );
}
