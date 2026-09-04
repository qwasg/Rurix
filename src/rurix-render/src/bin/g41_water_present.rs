//! G41 水面实时窗口呈现 harness(门 `g41.water.surface`)。
//!
//! 复现 HPWater(<https://github.com/AshenOneArt/HPWater>,Unity HDRP 水体渲染
//! 系统,MPL-2.0)所刻画的**技术方案**并接进真窗口实时车道:解析泻湖场景 +
//! 交互水波 + 水体 GBuffer + 屏幕空间折射 + 体积吸收散射 + 焦散 + 反射。
//!
//! **clean-room 纪律**:本车道与 `kernels/g41_water_*.rx` 只按公开算法族重新
//! 推导实现,不含 HPWater 仓库的源码文本(HLSL → `.rx` 逐行翻译在 MPL-2.0
//! §1.10 下构成 "Modification",与本仓库 `MIT OR Apache-2.0` 授权面冲突)。
//! 技术出处与许可分析见 `rfcs/0050-water-surface-rendering.md` §2 / §7;
//! 先例 = G40 对 HPVolumeCloud 的同构处理。
//!
//! ## 车道(五 pass 持久 session + swapchain 真窗口 present)
//!
//! ```text
//! g41_water_wave     (波方程 256² ping-pong;三缓冲经 binding_overrides 轮转)
//! g41_water_scene    (解析泻湖 ray march → 场景色 + 真视深)
//! g41_water_blur ×2  (2× box 降采样链 L1 / L2;替代无硬件 mip 的散射模糊)
//! g41_water_surface  (水面 GBuffer + 折射 + 体积 + 焦散 + 反射 + 泡沫合成)
//! g41_water_encode   (曝光 → ACES → sRGB → BGRA8;回读 8.3MB @1080p)
//!   → ExternalImagePresent swapchain
//! ```
//!
//! device 侧编码是刻意的:回读量从 w×h×12B 降到 w×h×4B(G31 A1 教训同律)。
//!
//! ## 与冻结面的关系(0-byte 纪律)
//!
//! 本 bin 与 `g31_window_present` / `g14_3_lane_body` / `display/` **零共享**——
//! 不 include 共享体、不复用生产 kernel、不触碰任何冻结 digest 锚。天空复用
//! G40 已入树的 `world::sky`(程序化 Rayleigh + Mie + 臭氧),水面 host 金标准
//! 为 `world::water_surface`,与 M113 的 `world::water`(RXS-0366 冻结带)分属
//! 两个模块、零耦合。
//!
//! ## 三态纪律
//!
//! 无 Vulkan 设备 / 缺 SPV ⇒ 打印 `skipped_dev_env` 退 0;`RURIX_REQUIRE_REAL=1`
//! 下翻硬红退 1。`--headless` 跳过窗口只跑渲染(CI 面)。
//!
//! ## 用法
//!
//! ```text
//! g41_water_present [--preset noon|clear|golden|sunset]
//!     [--width 1280] [--height 720] [--frames N] [--fov 60] [--ev F]
//!     [--water on|off] [--refraction on|off] [--volume on|off]
//!     [--caustics on|off] [--dispersion on|off] [--foam on|off] [--reflection on|off]
//!     [--depth F] [--shore-radius F] [--absorb r,g,b] [--scatter r,g,b]
//!     [--roughness F] [--wave-amp F] [--wave-speed F] [--wave-damping F]
//!     [--drops "帧:u,v,I[,r];…"] [--warmup N] [--seed N]
//!     [--cam-orbit] [--spv-dir <dir>] [--headless] [--dump <out.png>] [--digest]
//!     [--env-lut <skylut>] [--debug-view N] [--dump-raw <base> --dump-raw-every <n>]
//! ```
//!
//! 交互(窗口臂):`WASD` 平移 / `QE` 升降 / 方向键转视角 / 鼠标拖动转视角 /
//! `空格` 在视线落水点投一滴水 / `-` `=` 曝光 ±0.25 EV / `ESC` 退出。

#![forbid(unsafe_code)]

use image_io::{ImageBuffer, ImageFormat, Rgb, encode as encode_image};
use rurix_render::world::sky::{SKY_LUT_H, SKY_LUT_W, Sky, bake_sky_view_lut, preset_by_name};
use rurix_render::world::water_surface::{
    LagoonScene, NOISE2D_DIM, WATER_PARAM_COUNT, WAVE_DIM, WAVE_PARAM_COUNT, WaterArms,
    WaterCamera, WaterParams, WaveDrop, WaveParams, WaveSim, bake_noise2d, bake_obstacle_field,
    canonical_drops, pack_water_params, pack_wave_params, parse_drop_script, wave_digest,
};
use rurix_rt::render_exec::{
    Bindings, BufferDesc, BufferUsage, ComputePass, DeviceFrameSession, DispatchSpec, FrameUpdate,
    Pass, Readback, ResourceDesc, StableResourceId, TargetState,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

const TAG: &str = "G41_WATER";

// ── 资源下标(声明序 = 绑定序;与五个 kernel 的形参序逐字对应)──────────────
const R_WPARAMS: u32 = 0;
const R_WAVE_A: u32 = 1;
const R_WAVE_B: u32 = 2;
const R_WAVE_C: u32 = 3;
const R_OBSTACLE: u32 = 4;
const R_PARAMS: u32 = 5;
const R_NOISE2D: u32 = 6;
const R_SKY_LUT: u32 = 7;
const R_SCENE_COLOR: u32 = 8;
const R_SCENE_DEPTH: u32 = 9;
const R_BPARAMS1: u32 = 10;
const R_BLUR1: u32 = 11;
const R_BPARAMS2: u32 = 12;
const R_BLUR2: u32 = 13;
const R_OUT_COLOR: u32 = 14;
const R_EPARAMS: u32 = 15;
const R_BGRA: u32 = 16;
const R_COUNT: usize = 17;

// ── pass 下标(冻结序)────────────────────────────────────────────────────
const P_WAVE: u32 = 0;
const P_SCENE: u32 = 1;
const P_BLUR1: u32 = 2;
const P_BLUR2: u32 = 3;
const P_SURFACE: u32 = 4;
const P_ENCODE: u32 = 5;

/// 三缓冲波场轮转表(下标 = 帧 parity mod 3)。
const WAVE_RING: [u32; 3] = [R_WAVE_A, R_WAVE_B, R_WAVE_C];

// ── 屏障计划(逐 pass 保守超集:触达资源全声明为 storage 读写)──────────────
const PLAN_WAVE: &[(u32, TargetState)] = &[
    (R_WPARAMS, TargetState::StorageReadWrite),
    (R_WAVE_A, TargetState::StorageReadWrite),
    (R_WAVE_B, TargetState::StorageReadWrite),
    (R_WAVE_C, TargetState::StorageReadWrite),
    (R_OBSTACLE, TargetState::StorageReadWrite),
];
const PLAN_SCENE: &[(u32, TargetState)] = &[
    (R_PARAMS, TargetState::StorageReadWrite),
    (R_NOISE2D, TargetState::StorageReadWrite),
    (R_SKY_LUT, TargetState::StorageReadWrite),
    (R_SCENE_COLOR, TargetState::StorageReadWrite),
    (R_SCENE_DEPTH, TargetState::StorageReadWrite),
];
const PLAN_BLUR1: &[(u32, TargetState)] = &[
    (R_BPARAMS1, TargetState::StorageReadWrite),
    (R_SCENE_COLOR, TargetState::StorageReadWrite),
    (R_BLUR1, TargetState::StorageReadWrite),
];
const PLAN_BLUR2: &[(u32, TargetState)] = &[
    (R_BPARAMS2, TargetState::StorageReadWrite),
    (R_BLUR1, TargetState::StorageReadWrite),
    (R_BLUR2, TargetState::StorageReadWrite),
];
const PLAN_SURFACE: &[(u32, TargetState)] = &[
    (R_PARAMS, TargetState::StorageReadWrite),
    (R_WAVE_A, TargetState::StorageReadWrite),
    (R_WAVE_B, TargetState::StorageReadWrite),
    (R_WAVE_C, TargetState::StorageReadWrite),
    (R_NOISE2D, TargetState::StorageReadWrite),
    (R_SKY_LUT, TargetState::StorageReadWrite),
    (R_SCENE_COLOR, TargetState::StorageReadWrite),
    (R_BLUR1, TargetState::StorageReadWrite),
    (R_BLUR2, TargetState::StorageReadWrite),
    (R_SCENE_DEPTH, TargetState::StorageReadWrite),
    (R_OUT_COLOR, TargetState::StorageReadWrite),
];
const PLAN_ENCODE: &[(u32, TargetState)] = &[
    (R_OUT_COLOR, TargetState::StorageReadWrite),
    (R_EPARAMS, TargetState::StorageReadWrite),
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
    warmup: u32,
    fov_deg: f32,
    ev: f32,
    water: bool,
    arms: WaterArms,
    scene: LagoonScene,
    wp: WaterParams,
    wave: WaveParams,
    drops: Vec<WaveDrop>,
    seed: u64,
    cam_orbit: bool,
    spv_dir: PathBuf,
    headless: bool,
    debug_view: u32,
    env_lut: Option<PathBuf>,
    dump: Option<PathBuf>,
    dump_raw: Option<PathBuf>,
    dump_raw_every: Option<u32>,
    digest: bool,
}

fn parse_onoff(flag: &str, v: &str) -> bool {
    match v {
        "on" => true,
        "off" => false,
        s => fail(&format!("{flag} 闭集 on|off,得到 `{s}`")),
    }
}

fn parse_rgb(flag: &str, v: &str) -> [f32; 3] {
    let p: Vec<f32> = v
        .split(',')
        .map(|t| {
            t.trim()
                .parse()
                .unwrap_or_else(|_| fail(&format!("{flag} 分量非法")))
        })
        .collect();
    if p.len() != 3 {
        fail(&format!("{flag} 须为 r,g,b 三分量"));
    }
    [p[0], p[1], p[2]]
}

fn parse_args() -> Args {
    let root = workspace_root();
    let mut a = Args {
        preset: "golden".to_owned(),
        width: 1280,
        height: 720,
        frames: None,
        warmup: 90,
        fov_deg: 55.0,
        ev: 0.0,
        water: true,
        arms: WaterArms::default(),
        scene: LagoonScene::default(),
        wp: WaterParams::default(),
        wave: WaveParams::default(),
        drops: canonical_drops(),
        seed: 0x51ee_d001,
        cam_orbit: false,
        spv_dir: root.join(".tmp/g41/spv"),
        headless: false,
        debug_view: 0,
        env_lut: None,
        dump: None,
        dump_raw: None,
        dump_raw_every: None,
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
        let numf = |i: usize, name: &str| -> f32 {
            need(i)
                .parse()
                .unwrap_or_else(|_| fail(&format!("{name} 非法")))
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
            "--warmup" => {
                a.warmup = need(i).parse().unwrap_or_else(|_| fail("--warmup 非法"));
                i += 1;
            }
            "--fov" => {
                a.fov_deg = numf(i, "--fov");
                i += 1;
            }
            "--ev" => {
                a.ev = numf(i, "--ev");
                i += 1;
            }
            "--water" => {
                a.water = parse_onoff("--water", &need(i));
                i += 1;
            }
            "--refraction" => {
                a.arms.refraction = parse_onoff("--refraction", &need(i));
                i += 1;
            }
            "--volume" => {
                a.arms.volume = parse_onoff("--volume", &need(i));
                i += 1;
            }
            "--caustics" => {
                a.arms.caustics = parse_onoff("--caustics", &need(i));
                i += 1;
            }
            "--dispersion" => {
                a.arms.dispersion = parse_onoff("--dispersion", &need(i));
                i += 1;
            }
            "--foam" => {
                a.arms.foam = parse_onoff("--foam", &need(i));
                i += 1;
            }
            "--reflection" => {
                a.arms.reflection = parse_onoff("--reflection", &need(i));
                i += 1;
            }
            "--depth" => {
                a.scene.max_depth_m = numf(i, "--depth");
                i += 1;
            }
            "--shore-radius" => {
                a.scene.shore_radius_m = numf(i, "--shore-radius");
                i += 1;
            }
            "--absorb" => {
                a.wp.absorption = parse_rgb("--absorb", &need(i));
                i += 1;
            }
            "--scatter" => {
                a.wp.scattering = parse_rgb("--scatter", &need(i));
                i += 1;
            }
            "--roughness" => {
                a.wp.roughness = numf(i, "--roughness");
                i += 1;
            }
            "--wave-amp" => {
                a.wp.wave_amplitude_m = numf(i, "--wave-amp");
                i += 1;
            }
            "--wave-speed" => {
                a.wave.speed = numf(i, "--wave-speed");
                i += 1;
            }
            "--wave-damping" => {
                a.wave.damping = numf(i, "--wave-damping");
                i += 1;
            }
            "--drops" => {
                a.drops = parse_drop_script(&need(i))
                    .unwrap_or_else(|e| fail(&format!("--drops 解析失败: {e}")));
                i += 1;
            }
            "--seed" => {
                a.seed = need(i).parse().unwrap_or_else(|_| fail("--seed 非法"));
                i += 1;
            }
            "--cam-orbit" => a.cam_orbit = true,
            "--spv-dir" => {
                a.spv_dir = PathBuf::from(need(i));
                i += 1;
            }
            "--env-lut" => {
                a.env_lut = Some(PathBuf::from(need(i)));
                i += 1;
            }

            "--dump-raw" => {
                a.dump_raw = Some(PathBuf::from(need(i)));
                i += 1;
            }

            "--dump-raw-every" => {
                a.dump_raw_every = Some(
                    need(i)
                        .parse()
                        .unwrap_or_else(|_| fail("--dump-raw-every 非法")),
                );
                i += 1;
            }

            "--dump" => {
                a.dump = Some(PathBuf::from(need(i)));
                i += 1;
            }
            "--debug-view" => {
                a.debug_view = need(i)
                    .parse()
                    .unwrap_or_else(|_| fail("--debug-view 非法"));
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
    // 模糊链需要 4 的整除性(两级 2× 降采样)。
    if a.width % 4 != 0 || a.height % 4 != 0 {
        fail("宽高须为 4 的倍数(两级 2× 模糊链)");
    }
    if !a.water {
        a.arms = WaterArms::all_off();
    }
    a.scene
        .validate()
        .unwrap_or_else(|e| fail(&format!("场景非法: {e}")));
    a.wave
        .validate()
        .unwrap_or_else(|e| fail(&format!("波参数非法: {e}")));
    a
}

fn read_spv(path: &Path) -> Vec<u8> {
    match std::fs::read(path) {
        Ok(b) if b.len() >= 4 && b.len() % 4 == 0 && b[0..4] == [0x03, 0x02, 0x23, 0x07] => b,
        Ok(_) => fail(&format!("SPV 非法(magic/对齐): {}", path.display())),
        Err(e) => skip_or_fail(&format!(
            "SPV 不在位 {}: {e}(先跑 `py -3 ci/g41_water_smoke.py --build-spv`)",
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

/// 逐帧上传目标(须 host-visible)。
fn upload_buf(bytes: &[u8]) -> ResourceDesc<'_> {
    ResourceDesc::Buffer(BufferDesc {
        size: bytes.len() as u64,
        usage: STORAGE,
        data: Some(bytes),
        device_local: false,
    })
}

/// 创建期一次上传、之后只读的大表。
fn static_buf(bytes: &[u8]) -> ResourceDesc<'_> {
    ResourceDesc::Buffer(BufferDesc {
        size: bytes.len() as u64,
        usage: STORAGE,
        data: Some(bytes),
        device_local: true,
    })
}

/// 全 GPU 驻留的中间/输出缓冲。
fn scratch_buf<'a>(size: u64) -> ResourceDesc<'a> {
    ResourceDesc::Buffer(BufferDesc {
        size,
        usage: STORAGE,
        data: None,
        device_local: true,
    })
}

/// 飞行相机(yaw/pitch 度制;世界 Y-up)。
struct FlyCamera {
    pos: [f32; 3],
    yaw_deg: f32,
    pitch_deg: f32,
}

impl FlyCamera {
    fn basis(&self, aspect: f32, tan_half: f32) -> WaterCamera {
        let y = self.yaw_deg.to_radians();
        let p = self.pitch_deg.clamp(-89.0, 89.0).to_radians();
        let forward = [p.cos() * y.sin(), p.sin(), p.cos() * y.cos()];
        let right = {
            let r = [forward[2], 0.0, -forward[0]];
            let l = (r[0] * r[0] + r[2] * r[2]).sqrt().max(1e-6);
            [r[0] / l, 0.0, r[2] / l]
        };
        // up = cross(forward, right)(注意次序:cross(right, forward) 会得到 −up)。
        let up = [
            forward[1] * right[2] - forward[2] * right[1],
            forward[2] * right[0] - forward[0] * right[2],
            forward[0] * right[1] - forward[1] * right[0],
        ];
        WaterCamera {
            origin: self.pos,
            forward,
            right,
            up,
            tan_half_fov: tan_half,
            aspect,
        }
    }
}

/// 相机视线与水面的交点 → 波网格归一化坐标(空格投水滴用)。
fn aim_to_wave_uv(
    cam: &FlyCamera,
    scene: &LagoonScene,
    tan_half: f32,
    aspect: f32,
) -> Option<(f32, f32)> {
    let c = cam.basis(aspect, tan_half);
    let d = c.forward;
    if d[1] >= -1e-3 {
        return None;
    }
    let t = (scene.water_level - c.origin[1]) / d[1];
    if t <= 0.0 {
        return None;
    }
    let x = c.origin[0] + d[0] * t;
    let z = c.origin[2] + d[2] * t;
    let u = (x - scene.center_x) / scene.wave_extent_m + 0.5;
    let v = (z - scene.center_z) / scene.wave_extent_m + 0.5;
    if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
        Some((u, v))
    } else {
        None
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

    // ── host 事实源建造(天空 / 噪声 / 障碍场;一次性)─────────────────────
    let t_bake = Instant::now();
    let sky = Sky::new(preset);
    // 环境光:缺省用程序化天空;`--env-lut` 换成实拍水景 HDRI 烘焙的同格式 LUT
    // (Poly Haven CC0,由 `artifacts/day_0903_water/tools/fetch_env_hdri.py`
    // 下载并烘焙;二进制留缓存根不入 git)。太阳方向/色仍取程序化天空——LUT
    // 只替换天空背景与水面反射的环境项,如实登记。
    let sky_lut = match args.env_lut.as_ref() {
        None => bake_sky_view_lut(&sky),
        Some(p) => {
            let b = std::fs::read(p)
                .unwrap_or_else(|e| fail(&format!("--env-lut 读失败 {}: {e}", p.display())));
            if b.len() < 8 {
                fail("--env-lut 文件过短");
            }
            let lw = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
            let lh = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize;
            if lw != SKY_LUT_W || lh != SKY_LUT_H {
                fail(&format!(
                    "--env-lut 维度 {lw}×{lh} != 期望 {SKY_LUT_W}×{SKY_LUT_H}"
                ));
            }
            let want = SKY_LUT_W * SKY_LUT_H * 3;
            if b.len() != 8 + want * 4 {
                fail(&format!(
                    "--env-lut 长度 {} != 期望 {}",
                    b.len(),
                    8 + want * 4
                ));
            }
            println!("{TAG}: env_lut {} ({lw}×{lh})", p.display());
            b[8..]
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect()
        }
    };
    let noise2d = bake_noise2d(args.seed);
    let obstacle = bake_obstacle_field(WAVE_DIM, &args.scene);
    let bake_ms = t_bake.elapsed().as_secs_f64() * 1000.0;

    // host 波场镜像(金标准;与 device 同一参数面、同一注入次序)。
    let mut mirror =
        WaveSim::new(WAVE_DIM, args.wave).unwrap_or_else(|e| fail(&format!("波场建造失败: {e}")));
    mirror.fill_obstacles_from_scene(&args.scene);

    // ── SPV 装载 ──────────────────────────────────────────────────────────
    let spv_wave = read_spv(&args.spv_dir.join("g41_water_wave.spv"));
    let spv_scene = read_spv(&args.spv_dir.join("g41_water_scene.spv"));
    let spv_blur = read_spv(&args.spv_dir.join("g41_water_blur.spv"));
    let spv_surface = read_spv(&args.spv_dir.join("g41_water_surface.spv"));
    let spv_encode = read_spv(&args.spv_dir.join("g41_water_encode.spv"));

    let (w, h) = (args.width, args.height);
    let px_count = u64::from(w) * u64::from(h);
    let color_bytes = px_count * 12;
    let depth_bytes = px_count * 4;
    let bgra_bytes = px_count * 4;
    let b1_px = u64::from(w / 2) * u64::from(h / 2);
    let b2_px = u64::from(w / 4) * u64::from(h / 4);
    let wave_bytes = (WAVE_DIM * WAVE_DIM * 4) as u64;

    // ── 相机(俯瞰泻湖,略高于水面 ⇒ 同帧看到深水/浅滩/干岸三段梯度)────────
    let mut cam = FlyCamera {
        pos: [0.0, 13.0, 44.0],
        yaw_deg: 180.0,
        pitch_deg: -19.0,
    };
    let tan_half = (args.fov_deg.to_radians() * 0.5).tan();
    let aspect = w as f32 / h as f32;
    // 曝光 = 显示增益 × 2^EV 偏置。
    //
    // **不用 `SkyPreset::ev100`**:实测 `world::sky` 的 `sun_color` 已是**归一化**
    // 辐亮度(clear 档 ≈ (0.9, 0.8, 0.6)、ambient_top ≈ 0.01),不是 10⁴ 级的
    // 物理辐亮度;若按 `1/(1.2·2^ev100)` 换算会整幅压黑约 10⁵ 倍。`ev100` 是该
    // 预设标定源资产的记录值,供参照而非本车道的曝光输入(G40 同律用固定增益)。
    // 增益 1.0 由实测定标:`clear` 档 `--water off` 全幅 present 均值 ≈ 103/255
    // (中间调),增益 4.0 时均值 192(整幅过曝)。
    const DISPLAY_GAIN: f32 = 1.0;
    // `SkyPreset::ev100` 按**相对**曝光补偿消费:它是各档标定源资产的记录 EV,
    // 低日角档(golden 12.0 / sunset 10.5)本就该比 clear(14.5)开大光圈。
    // 不这么补时低日角档整幅压黑(实测 golden 均值 27.6、sunset 14.9 /255)。
    const EV100_REF: f32 = 14.5;
    let exposure_for =
        |ev_bias: f32| -> f32 { 2.0f32.powf(EV100_REF - preset.ev100 + ev_bias) * DISPLAY_GAIN };
    let exposure = exposure_for(args.ev);

    // ── 初始上传字节 ──────────────────────────────────────────────────────
    let wparams0 = pack_wave_params(WAVE_DIM, &args.wave, &[]);
    let params0 = pack_water_params(
        &args.wp,
        &args.scene,
        &sky,
        &cam.basis(aspect, tan_half),
        w,
        h,
        0,
        0.0,
        exposure,
        args.arms,
        160,
        260.0,
        SKY_LUT_W,
        SKY_LUT_H,
        args.debug_view,
    );
    debug_assert_eq!(params0.len(), WATER_PARAM_COUNT);
    debug_assert_eq!(wparams0.len(), WAVE_PARAM_COUNT);

    let wparams_bytes = f32s_to_bytes(&wparams0);
    let params_bytes = f32s_to_bytes(&params0);
    let obstacle_bytes = f32s_to_bytes(&obstacle);
    let noise_bytes = f32s_to_bytes(&noise2d);
    let sky_bytes = f32s_to_bytes(&sky_lut);
    let bparams1_bytes = f32s_to_bytes(&[w as f32, h as f32, (w / 2) as f32, (h / 2) as f32]);
    let bparams2_bytes = f32s_to_bytes(&[
        (w / 2) as f32,
        (h / 2) as f32,
        (w / 4) as f32,
        (h / 4) as f32,
    ]);
    let dbg_raw = f32::from(u8::from(args.debug_view != 0));
    let eparams_bytes = f32s_to_bytes(&[w as f32, h as f32, exposure, 1.0 / 2.2, dbg_raw]);
    let wave_zero = vec![0u8; wave_bytes as usize];

    let resources = vec![
        upload_buf(&wparams_bytes),  // 0  R_WPARAMS
        static_buf(&wave_zero),      // 1  R_WAVE_A
        static_buf(&wave_zero),      // 2  R_WAVE_B
        static_buf(&wave_zero),      // 3  R_WAVE_C
        static_buf(&obstacle_bytes), // 4  R_OBSTACLE
        upload_buf(&params_bytes),   // 5  R_PARAMS
        static_buf(&noise_bytes),    // 6  R_NOISE2D
        static_buf(&sky_bytes),      // 7  R_SKY_LUT
        scratch_buf(color_bytes),    // 8  R_SCENE_COLOR
        scratch_buf(depth_bytes),    // 9  R_SCENE_DEPTH
        static_buf(&bparams1_bytes), // 10 R_BPARAMS1
        scratch_buf(b1_px * 12),     // 11 R_BLUR1
        static_buf(&bparams2_bytes), // 12 R_BPARAMS2
        scratch_buf(b2_px * 12),     // 13 R_BLUR2
        scratch_buf(color_bytes),    // 14 R_OUT_COLOR
        upload_buf(&eparams_bytes),  // 15 R_EPARAMS
        scratch_buf(bgra_bytes),     // 16 R_BGRA
    ];
    assert_eq!(resources.len(), R_COUNT, "资源表长度须与下标闭集一致");

    let groups_full = [w.div_ceil(8), h.div_ceil(8), 1];
    let groups_b1 = [(w / 2).div_ceil(8), (h / 2).div_ceil(8), 1];
    let groups_b2 = [(w / 4).div_ceil(8), (h / 4).div_ceil(8), 1];
    let groups_wave = [
        (WAVE_DIM as u32).div_ceil(8),
        (WAVE_DIM as u32).div_ceil(8),
        1,
    ];

    let cp = |name: &'static str,
              spirv: &'static [u8],
              dispatch: DispatchSpec,
              bufs: Vec<u32>|
     -> Pass<'static> {
        Pass::Compute(ComputePass {
            name,
            spirv,
            entry: None,
            dispatch,
            bindings: Bindings {
                storage_buffers: bufs,
                ..Bindings::default()
            },
        })
    };
    // SPV 生命周期须覆盖 session;泄漏为进程级一次性(与 G40 同律)。
    let spv_wave: &'static [u8] = Box::leak(spv_wave.into_boxed_slice());
    let spv_scene: &'static [u8] = Box::leak(spv_scene.into_boxed_slice());
    let spv_blur: &'static [u8] = Box::leak(spv_blur.into_boxed_slice());
    let spv_surface: &'static [u8] = Box::leak(spv_surface.into_boxed_slice());
    let spv_encode: &'static [u8] = Box::leak(spv_encode.into_boxed_slice());

    // 声明期绑定 = 帧 0 的轮转(prev=A, cur=B, next=C)。
    let passes = vec![
        cp(
            "g41_water_wave",
            spv_wave,
            DispatchSpec::Direct(groups_wave),
            vec![R_WPARAMS, R_WAVE_B, R_WAVE_A, R_OBSTACLE, R_WAVE_C],
        ),
        cp(
            "g41_water_scene",
            spv_scene,
            DispatchSpec::Direct(groups_full),
            vec![R_PARAMS, R_NOISE2D, R_SKY_LUT, R_SCENE_COLOR, R_SCENE_DEPTH],
        ),
        cp(
            "g41_water_blur_l1",
            spv_blur,
            DispatchSpec::Direct(groups_b1),
            vec![R_BPARAMS1, R_SCENE_COLOR, R_BLUR1],
        ),
        cp(
            "g41_water_blur_l2",
            spv_blur,
            DispatchSpec::Direct(groups_b2),
            vec![R_BPARAMS2, R_BLUR1, R_BLUR2],
        ),
        cp(
            "g41_water_surface",
            spv_surface,
            DispatchSpec::Direct(groups_full),
            vec![
                R_PARAMS,
                R_WAVE_C,
                R_NOISE2D,
                R_SKY_LUT,
                R_SCENE_COLOR,
                R_BLUR1,
                R_BLUR2,
                R_SCENE_DEPTH,
                R_OUT_COLOR,
            ],
        ),
        cp(
            "g41_water_encode",
            spv_encode,
            DispatchSpec::Direct(groups_full),
            vec![R_OUT_COLOR, R_EPARAMS, R_BGRA],
        ),
    ];
    // pass 序即契约:`binding_overrides` 按下标寻址,错序会把波场绑到错的 pass。
    let pass_name_at = |i: u32| -> &'static str {
        match &passes[i as usize] {
            Pass::Compute(c) => c.name,
            _ => "non-compute",
        }
    };
    assert_eq!(pass_name_at(P_WAVE), "g41_water_wave");
    assert_eq!(pass_name_at(P_SCENE), "g41_water_scene");
    assert_eq!(pass_name_at(P_BLUR1), "g41_water_blur_l1");
    assert_eq!(pass_name_at(P_BLUR2), "g41_water_blur_l2");
    assert_eq!(pass_name_at(P_SURFACE), "g41_water_surface");
    assert_eq!(pass_name_at(P_ENCODE), "g41_water_encode");

    let barriers: Vec<&[(u32, TargetState)]> = vec![
        PLAN_WAVE,
        PLAN_SCENE,
        PLAN_BLUR1,
        PLAN_BLUR2,
        PLAN_SURFACE,
        PLAN_ENCODE,
    ];
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
        match rurix_rt::vk::ExternalImagePresent::create(w, h, "Rurix G41 — 水面", true) {
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

    let sc = sky.sun_direction();
    let sl = sky.sun_color();
    let (at, _ab) = sky.ambient_probe();
    println!(
        "{TAG}: start preset={} sun_elev={:.2}deg ref={} {w}x{h} water={} depth={:.1}m \
         wave={WAVE_DIM}² noise={NOISE2D_DIM}² ev100={:.2} exposure={exposure:.3e} bake_ms={bake_ms:.1}",
        preset.name,
        preset.sun_elevation_deg,
        preset.reference_slug,
        if args.water { "on" } else { "off" },
        args.scene.max_depth_m,
        preset.ev100,
    );
    println!(
        "{TAG}: radiometry sun_dir=({:.3},{:.3},{:.3}) sun_color=({:.1},{:.1},{:.1}) \
         ambient_top=({:.2},{:.2},{:.2})",
        sc[0], sc[1], sc[2], sl[0], sl[1], sl[2], at[0], at[1], at[2],
    );
    if args.debug_view != 0 {
        for r in 0..2usize {
            let b = 96 + r * 8;
            println!(
                "{TAG}: rock[{r}] packed c=({:.2},{:.2},{:.2}) r={:.2} albedo=({:.2},{:.2},{:.2})",
                params0[b],
                params0[b + 1],
                params0[b + 2],
                params0[b + 3],
                params0[b + 4],
                params0[b + 5],
                params0[b + 6],
            );
        }
    }

    // ── 帧循环 ────────────────────────────────────────────────────────────
    let total = args
        .frames
        .unwrap_or(if window.is_some() { u32::MAX } else { 1 });
    let mut frame = 0u32;
    let mut ev_bias = args.ev;
    let mut last_bgra: Vec<u8> = Vec::new();
    let mut render_ms_sum = 0.0f64;
    let mut render_ms_count = 0u32;
    let mut manual_drops: Vec<WaveDrop> = Vec::new();
    let mut space_was_down = false;
    let t_loop = Instant::now();
    let dt = 1.0f32 / 60.0;

    // warmup:让脚本波源先跑起来再计入出图(与 rain_night 同律)。
    let max_frames = total.saturating_add(args.warmup);

    while frame < max_frames {
        // 输入 → 相机 / 曝光 / 投水滴。
        if let Some(win) = window.as_mut() {
            let input = win.poll_input();
            if input.close_requested {
                break;
            }
            if input.minimized {
                continue;
            }
            let speed = 0.9f32;
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
                // 相机不下潜到水面以下(水下渲染不在本车道范围,如实拒)。
                cam.pos[1] = (cam.pos[1] - speed).max(args.scene.water_level + 0.35);
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
            // 空格:在视线落水点投一滴(边沿触发,避免按住时每帧刷屏)。
            let space = input.key(0x20);
            if space && !space_was_down {
                if let Some((u, v)) = aim_to_wave_uv(&cam, &args.scene, tan_half, aspect) {
                    manual_drops.push(WaveDrop {
                        frame,
                        u,
                        v,
                        intensity: 1.5,
                        radius: 6.0,
                    });
                }
            }
            space_was_down = space;
        } else if args.cam_orbit {
            // 离屏环绕(确定性;出短片用)。
            let a = (frame as f32) / (max_frames.max(1) as f32) * std::f32::consts::TAU;
            let r = 52.0;
            cam.pos = [r * a.sin(), 7.5 + 2.0 * (a * 2.0).sin(), r * a.cos()];
            cam.yaw_deg = 180.0 + a.to_degrees();
        }

        // 本帧波源 = 脚本 + 手动(截断到 device 定长槽位由 pack 负责)。
        let mut frame_drops: Vec<WaveDrop> = WaveSim::drops_for_frame(&args.drops, frame);
        frame_drops.extend(manual_drops.iter().filter(|d| d.frame == frame).copied());

        // host 金标准镜像同步推进(与 device 同一参数、同一注入次序)。
        mirror
            .step_with_drops(&frame_drops)
            .unwrap_or_else(|e| fail(&format!("host 波场步进失败: {e}")));

        // 三缓冲轮转:prev/cur/next。
        let i_prev = (frame % 3) as usize;
        let i_cur = ((frame + 1) % 3) as usize;
        let i_next = ((frame + 2) % 3) as usize;
        let r_prev = WAVE_RING[i_prev];
        let r_cur = WAVE_RING[i_cur];
        let r_next = WAVE_RING[i_next];

        let camf = cam.basis(aspect, tan_half);
        let p = pack_water_params(
            &args.wp,
            &args.scene,
            &sky,
            &camf,
            w,
            h,
            frame,
            (frame as f32) * dt,
            exposure_for(ev_bias),
            args.arms,
            160,
            260.0,
            SKY_LUT_W,
            SKY_LUT_H,
            args.debug_view,
        );
        let wpk = pack_wave_params(WAVE_DIM, &args.wave, &frame_drops);
        let exp_now = exposure_for(ev_bias);

        let update = FrameUpdate {
            buffer_uploads: vec![
                (
                    StableResourceId(u64::from(R_PARAMS) + 1),
                    0,
                    f32s_to_bytes(&p),
                ),
                (
                    StableResourceId(u64::from(R_WPARAMS) + 1),
                    0,
                    f32s_to_bytes(&wpk),
                ),
                (
                    StableResourceId(u64::from(R_EPARAMS) + 1),
                    0,
                    f32s_to_bytes(&[w as f32, h as f32, exp_now, 1.0 / 2.2, dbg_raw]),
                ),
            ],
            // 波场三缓冲轮转:wave pass 读 cur/prev 写 next;surface pass 读 next。
            binding_overrides: vec![
                (
                    P_WAVE,
                    Bindings {
                        storage_buffers: vec![R_WPARAMS, r_cur, r_prev, R_OBSTACLE, r_next],
                        ..Bindings::default()
                    },
                ),
                (
                    P_SURFACE,
                    Bindings {
                        storage_buffers: vec![
                            R_PARAMS,
                            r_next,
                            R_NOISE2D,
                            R_SKY_LUT,
                            R_SCENE_COLOR,
                            R_BLUR1,
                            R_BLUR2,
                            R_SCENE_DEPTH,
                            R_OUT_COLOR,
                        ],
                        ..Bindings::default()
                    },
                ),
            ],
            // `execute_with_frame_update` 路径下 `None` = 不回读,须显式点名。
            readback_subset: Some(vec![0]),
            ..FrameUpdate::default()
        };

        let t_frame = Instant::now();
        let expected = match session.next_provenance_with_update(&update) {
            Ok(pv) => pv,
            Err(e) => fail(&format!("帧 {frame} provenance 预推失败: {e}")),
        };
        let out = match session.execute_with_frame_update(&expected, &update) {
            Ok(o) => o,
            Err(e) => fail(&format!("帧 {frame} 提交失败: {e}")),
        };
        let ms = t_frame.elapsed().as_secs_f64() * 1000.0;
        if frame >= args.warmup {
            render_ms_sum += ms;
            render_ms_count += 1;
        }
        last_bgra = out.readbacks.into_iter().next().unwrap_or_default();

        // 逐帧 raw 落盘(短片腿):`<base>.f<帧号 4 位>` = w/h u32 LE 头 + BGRA8,
        // 与 g31_window_present / day_0902_rain_night `--dump-present-every` 逐字同
        // 布局,`make_*_clip.py` 直通。帧号含 warmup(转片时按 --warmup-skip 剔)。
        if let (Some(base), Some(n)) = (args.dump_raw.as_ref(), args.dump_raw_every) {
            if n > 0 && frame % n == 0 {
                let mut raw = Vec::with_capacity(8 + last_bgra.len());
                raw.extend_from_slice(&w.to_le_bytes());
                raw.extend_from_slice(&h.to_le_bytes());
                raw.extend_from_slice(&last_bgra);
                let p = PathBuf::from(format!("{}.f{frame:04}", base.display()));
                if let Some(dir) = p.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                std::fs::write(&p, &raw)
                    .unwrap_or_else(|e| fail(&format!("写 raw 帧 {}: {e}", p.display())));
            }
        }

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

    // ── 出图 ──────────────────────────────────────────────────────────────
    if let Some(path) = args.dump.as_ref() {
        if last_bgra.len() as u64 != bgra_bytes {
            fail("回读长度与 BGRA8 期望不符");
        }
        let mut buf = ImageBuffer::new(w, h, Rgb::new(0.0, 0.0, 0.0));
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
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

    // ── 呈现面统计(诊断腿:整幅过曝/全黑一眼可判,无需读回 PNG)──────────────
    if !last_bgra.is_empty() {
        let n = last_bgra.len() / 4;
        let mut sum = [0u64; 3];
        let mut mx = [0u8; 3];
        let mut mn = [255u8; 3];
        let mut sat = 0u64;
        for p in 0..n {
            // BGRA8。
            for (c, off) in [(0usize, 2usize), (1, 1), (2, 0)] {
                let v = last_bgra[p * 4 + off];
                sum[c] += u64::from(v);
                mx[c] = mx[c].max(v);
                mn[c] = mn[c].min(v);
            }
            if last_bgra[p * 4] == 255 && last_bgra[p * 4 + 1] == 255 && last_bgra[p * 4 + 2] == 255
            {
                sat += 1;
            }
        }
        let d = n as f64;
        println!(
            "{TAG}: present_stats mean=({:.1},{:.1},{:.1}) min=({},{},{}) max=({},{},{}) \
             saturated_px={:.2}%",
            sum[0] as f64 / d,
            sum[1] as f64 / d,
            sum[2] as f64 / d,
            mn[0],
            mn[1],
            mn[2],
            mx[0],
            mx[1],
            mx[2],
            sat as f64 / d * 100.0,
        );
    }

    let hexs = |d: [u8; 32]| -> String { d.iter().map(|b| format!("{b:02x}")).collect() };
    let digest = if args.digest {
        format!(
            " present_digest=sha256:{} wave_digest=sha256:{}",
            hexs(rurix_pkg::sha256::digest(&last_bgra)),
            hexs(wave_digest(&mirror)),
        )
    } else {
        String::new()
    };

    println!(
        "{TAG}: PASS preset={} frames={frame} warmup={} render_frame_ms={mean_ms:.3} fps={fps:.1} \
         wall_s={wall_s:.2} water={} refract={} volume={} caustic={} disp={} foam={} reflect={} \
         wave_energy={:.4}{digest}",
        preset.name,
        args.warmup,
        if args.water { "on" } else { "off" },
        if args.arms.refraction { "on" } else { "off" },
        if args.arms.volume { "on" } else { "off" },
        if args.arms.caustics { "on" } else { "off" },
        if args.arms.dispersion { "on" } else { "off" },
        if args.arms.foam { "on" } else { "off" },
        if args.arms.reflection { "on" } else { "off" },
        mirror.energy(),
    );
}
