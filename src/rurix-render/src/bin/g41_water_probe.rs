//! G41 波方程 device↔host 对拍探针(门 `g41.water.surface` 的 wave 腿)。
//!
//! 把 `kernels/g41_water_wave.rx` 在真设备上逐帧推进,与 host 金标准
//! [`rurix_render::world::water_surface::WaveSim`] **同参数、同注入次序**并行
//! 推进,逐格比对高度场。两者是同一套离散格式的两份独立实现(一份 `.rx`、
//! 一份 safe Rust),对拍即是"公式面逐字同源"这一说法的机器证据。
//!
//! ## 三缓冲轮转
//!
//! 与 `g41_water_present` 逐字同构:第 f 帧 prev = ring[f%3]、cur = ring[(f+1)%3]、
//! next = ring[(f+2)%3],经 `binding_overrides` 换绑。
//!
//! ## 容差(为什么不是位级相等)
//!
//! 波方程主体是纯 f32 加乘,已用 SPIR-V `NoContraction` 关掉 FMA 收缩
//! (见 [`spv_inject_no_contraction`]),但**高斯波源注入含 `exp`**——Vulkan 只
//! 要求 `OpExtInst Exp` 精度在若干 ULP 内,与 host libm 的实现不同源,故位级
//! 相等在本构型下**不可达**。实测归因链(90 帧 / 256²):
//!
//! | 状态 | max_abs_diff |
//! |---|---|
//! | 无 NoContraction | 1.4901161e-6 |
//! | 注入 NoContraction | 1.1920929e-6 |
//! | + host 除法形式对齐 | 见 evidence(冻结带) |
//!
//! 故判据取**measured 冻结带**:`--freeze` 把当次实测最大绝对差写入带文件,
//! 之后每次跑与带比对,超出即红。阈值由程序产出、不手写(P-09)。
//!
//! ## 三态
//!
//! 无 Vulkan / 缺 SPV ⇒ `skipped_dev_env` 退 0;`RURIX_REQUIRE_REAL=1` 翻硬红。
//!
//! ## 用法
//!
//! ```text
//! g41_water_probe [--frames 90] [--spv <wave.spv>] [--drops "帧:u,v,I[,r];…"]
//!                 [--band <band.json>] [--freeze] [--evidence <path>]
//! ```

#![forbid(unsafe_code)]

use rurix_render::world::water_surface::{
    LagoonScene, WAVE_DIM, WaveParams, WaveSim, bake_obstacle_field, canonical_drops,
    pack_wave_params, parse_drop_script, wave_digest,
};
use rurix_rt::render_exec::{
    Bindings, BufferDesc, BufferUsage, ComputePass, DeviceFrameSession, DispatchSpec, FrameUpdate,
    Pass, Readback, ResourceDesc, StableResourceId, TargetState,
};
use std::path::PathBuf;

const TAG: &str = "G41_WATER_PROBE";

const R_WPARAMS: u32 = 0;
const R_A: u32 = 1;
const R_B: u32 = 2;
const R_C: u32 = 3;
const R_OBSTACLE: u32 = 4;
const RING: [u32; 3] = [R_A, R_B, R_C];

const PLAN: &[(u32, TargetState)] = &[
    (R_WPARAMS, TargetState::StorageReadWrite),
    (R_A, TargetState::StorageReadWrite),
    (R_B, TargetState::StorageReadWrite),
    (R_C, TargetState::StorageReadWrite),
    (R_OBSTACLE, TargetState::StorageReadWrite),
];

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

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

fn f32s_to_bytes(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

fn bytes_to_f32s(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

const STORAGE: BufferUsage = BufferUsage {
    storage: true,
    uniform: false,
    vertex: false,
    indirect: false,
};

/// SPIR-V `NoContraction` 后处理:对全部 OpFAdd/OpFSub/OpFMul 结果 id 注入
/// `OpDecorate %id NoContraction`,禁驱动把 `mul+add` 收缩成 FMA——FMA 只舍入
/// 一次,与 host 的两次舍入逐 op 不等,正是 device↔host 位级对齐的先决条件。
///
/// **副本登记**:与 `g14_3_lane_body.rs::spv_inject_no_contraction`(单源)及
/// `g31_frame_cut_arm.rs::fc_spv_inject_no_contraction`(第三副本,该处已如实
/// 登记单源折叠留窗)字面同式。本文件不 include 共享体(G41 车道对冻结面零
/// 触碰),故再持一份;单源折叠一并归入 `rfcs/0050` §6 留窗表。
///
/// 实测归因:未注入时本 kernel 90 帧 device↔host `max_abs_diff = 1.49e-6`
/// (恰一 ULP@0.5 量级,699388 格不等);注入后见 evidence。
fn spv_inject_no_contraction(spv: &[u32]) -> Vec<u32> {
    let mut result_ids: Vec<u32> = Vec::new();
    let mut i = 5usize; // SPIR-V header 5 字
    let mut first_decorate: Option<usize> = None;
    let mut first_type: Option<usize> = None;
    while i < spv.len() {
        let w = spv[i];
        let wc = (w >> 16) as usize;
        let op = w & 0xFFFF;
        if wc == 0 || i + wc > spv.len() {
            fail("SPIR-V 指令流越界(NoContraction 注入)");
        }
        match op {
            71 if first_decorate.is_none() => first_decorate = Some(i),
            19..=39 if first_type.is_none() => first_type = Some(i),
            129 | 131 | 133 => result_ids.push(spv[i + 2]),
            _ => {}
        }
        i += wc;
    }
    let at = first_decorate
        .or(first_type)
        .unwrap_or_else(|| fail("SPIR-V 无 annotation/type 段锚(NoContraction 注入)"));
    let mut out = Vec::with_capacity(spv.len() + result_ids.len() * 3);
    out.extend_from_slice(&spv[..at]);
    for id in &result_ids {
        out.push(71u32 | (3 << 16)); // OpDecorate(wc=3)
        out.push(*id);
        out.push(42); // Decoration NoContraction
    }
    out.extend_from_slice(&spv[at..]);
    out
}

/// 字节流 → SPIR-V 字 → 注入 → 字节流。
fn load_spv_no_contraction(bytes: &[u8]) -> Vec<u8> {
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    spv_inject_no_contraction(&words)
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect()
}

fn main() {
    let root = workspace_root();
    let mut frames = 90u32;
    let mut spv_path = root.join(".tmp/g41/spv/g41_water_wave.spv");
    let mut drops = canonical_drops();
    let mut evidence: Option<PathBuf> = None;
    let mut band = root.join("artifacts/day_0903_water/g41_wave_band.json");
    let mut freeze = false;

    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < argv.len() {
        let need = |i: usize| -> String {
            argv.get(i + 1)
                .cloned()
                .unwrap_or_else(|| fail(&format!("{} 缺参数", argv[i])))
        };
        match argv[i].as_str() {
            "--frames" => {
                frames = need(i).parse().unwrap_or_else(|_| fail("--frames 非法"));
                i += 1;
            }
            "--spv" => {
                spv_path = PathBuf::from(need(i));
                i += 1;
            }
            "--drops" => {
                drops = parse_drop_script(&need(i))
                    .unwrap_or_else(|e| fail(&format!("--drops 解析失败: {e}")));
                i += 1;
            }
            "--evidence" => {
                evidence = Some(PathBuf::from(need(i)));
                i += 1;
            }
            "--band" => {
                band = PathBuf::from(need(i));
                i += 1;
            }
            "--freeze" => freeze = true,
            s => fail(&format!("未知参数 `{s}`")),
        }
        i += 1;
    }

    let spv_raw = match std::fs::read(&spv_path) {
        Ok(b) if b.len() >= 4 && b[0..4] == [0x03, 0x02, 0x23, 0x07] => b,
        Ok(_) => fail(&format!("SPV 非法: {}", spv_path.display())),
        Err(e) => skip_or_fail(&format!("SPV 不在位 {}: {e}", spv_path.display())),
    };
    // 注入 NoContraction:位级对拍的先决条件(见该函数文档的实测归因)。
    let spv = load_spv_no_contraction(&spv_raw);

    let wave = WaveParams::default();
    let scene = LagoonScene::default();
    let mut mirror =
        WaveSim::new(WAVE_DIM, wave).unwrap_or_else(|e| fail(&format!("host 波场: {e}")));
    mirror.fill_obstacles_from_scene(&scene);
    let obstacle = bake_obstacle_field(WAVE_DIM, &scene);

    let n = WAVE_DIM * WAVE_DIM;
    let zero = vec![0u8; n * 4];
    let obstacle_bytes = f32s_to_bytes(&obstacle);
    let wp0 = f32s_to_bytes(&pack_wave_params(WAVE_DIM, &wave, &[]));

    let resources = vec![
        ResourceDesc::Buffer(BufferDesc {
            size: wp0.len() as u64,
            usage: STORAGE,
            data: Some(&wp0),
            device_local: false,
        }),
        // 三个波场缓冲须 host-visible:对拍要逐帧回读。
        ResourceDesc::Buffer(BufferDesc {
            size: zero.len() as u64,
            usage: STORAGE,
            data: Some(&zero),
            device_local: false,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: zero.len() as u64,
            usage: STORAGE,
            data: Some(&zero),
            device_local: false,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: zero.len() as u64,
            usage: STORAGE,
            data: Some(&zero),
            device_local: false,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: obstacle_bytes.len() as u64,
            usage: STORAGE,
            data: Some(&obstacle_bytes),
            device_local: true,
        }),
    ];

    let groups = [
        (WAVE_DIM as u32).div_ceil(8),
        (WAVE_DIM as u32).div_ceil(8),
        1,
    ];
    let passes = vec![Pass::Compute(ComputePass {
        name: "g41_water_wave",
        spirv: &spv,
        entry: None,
        dispatch: DispatchSpec::Direct(groups),
        bindings: Bindings {
            storage_buffers: vec![R_WPARAMS, R_B, R_A, R_OBSTACLE, R_C],
            ..Bindings::default()
        },
    })];
    let barriers: Vec<&[(u32, TargetState)]> = vec![PLAN];
    // 三个 ring 缓冲各注册一条 readback,按帧选取。
    let readbacks = vec![
        Readback::Buffer {
            res: R_A,
            offset: 0,
            size: (n * 4) as u64,
        },
        Readback::Buffer {
            res: R_B,
            offset: 0,
            size: (n * 4) as u64,
        },
        Readback::Buffer {
            res: R_C,
            offset: 0,
            size: (n * 4) as u64,
        },
    ];

    let mut session = match DeviceFrameSession::new(&resources, &passes, &barriers, &readbacks, 2) {
        Ok(s) => s,
        Err(e) => skip_or_fail(&format!("device session: {e}")),
    };

    let mut max_abs = 0.0f32;
    let mut worst_frame = 0u32;
    let mut mismatched = 0u64;

    for f in 0..frames {
        let frame_drops = WaveSim::drops_for_frame(&drops, f);
        mirror
            .step_with_drops(&frame_drops)
            .unwrap_or_else(|e| fail(&format!("host 步进: {e}")));

        let i_prev = (f % 3) as usize;
        let i_cur = ((f + 1) % 3) as usize;
        let i_next = ((f + 2) % 3) as usize;
        let (r_prev, r_cur, r_next) = (RING[i_prev], RING[i_cur], RING[i_next]);

        let update = FrameUpdate {
            buffer_uploads: vec![(
                StableResourceId(u64::from(R_WPARAMS) + 1),
                0,
                f32s_to_bytes(&pack_wave_params(WAVE_DIM, &wave, &frame_drops)),
            )],
            binding_overrides: vec![(
                0,
                Bindings {
                    storage_buffers: vec![R_WPARAMS, r_cur, r_prev, R_OBSTACLE, r_next],
                    ..Bindings::default()
                },
            )],
            // 只回读本帧写入的 next 缓冲(ring 下标 = readback 下标)。
            readback_subset: Some(vec![i_next as u32]),
            ..FrameUpdate::default()
        };

        let expected = session
            .next_provenance_with_update(&update)
            .unwrap_or_else(|e| fail(&format!("帧 {f} provenance: {e}")));
        let out = session
            .execute_with_frame_update(&expected, &update)
            .unwrap_or_else(|e| fail(&format!("帧 {f} 提交: {e}")));
        let dev = bytes_to_f32s(&out.readbacks.into_iter().next().unwrap_or_default());
        if dev.len() != n {
            fail(&format!("帧 {f} 回读长度 {} != {n}", dev.len()));
        }
        for (a, b) in dev.iter().zip(mirror.height().iter()) {
            let d = (a - b).abs();
            if d > 0.0 {
                mismatched += 1;
            }
            if d > max_abs {
                max_abs = d;
                worst_frame = f;
            }
        }
    }

    let hexs = |d: [u8; 32]| -> String { d.iter().map(|b| format!("{b:02x}")).collect() };
    let host_digest = hexs(wave_digest(&mirror));
    let bit_equal = max_abs == 0.0;

    // ── measured 冻结带 ───────────────────────────────────────────────────
    if freeze {
        let json = format!(
            "{{\n  \"schema\": \"rurix.g41.wave_band.v1\",\n  \
             \"freeze_rule\": \"max_abs_diff = g41_water_wave.rx(注入 NoContraction)与 host \
             WaveSim 同参数同注入次序推进 {frames} 帧后逐格高度场最大绝对差 measured 值;\
             位级相等不可达(高斯波源含 exp,Vulkan OpExtInst Exp 与 host libm 非同源),\
             故取 measured 带。禁手写。\",\n  \
             \"frames\": {frames},\n  \"dim\": {WAVE_DIM},\n  \
             \"max_abs_diff\": {max_abs:e},\n  \
             \"host_wave_digest\": \"sha256:{host_digest}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\"}}\n}}\n",
            std::env::consts::OS,
            std::env::consts::ARCH,
        );
        if let Some(d) = band.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        std::fs::write(&band, json).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
        println!(
            "{TAG}: 冻结带已落盘 {} (max_abs_diff={max_abs:e})",
            band.display()
        );
    }
    let frozen: Option<f32> = std::fs::read_to_string(&band).ok().and_then(|t| {
        let key = "\"max_abs_diff\": ";
        let s = t.find(key)? + key.len();
        let e = t[s..].find(',')? + s;
        t[s..e].trim().parse().ok()
    });
    let within_band = match frozen {
        Some(f) => max_abs <= f,
        None if freeze => true,
        None => fail(&format!(
            "冻结带 {} 不存在——先跑 `--freeze` 产 measured 带(禁手写 golden)",
            band.display()
        )),
    };

    if let Some(p) = &evidence {
        let json = format!(
            "{{\n  \"schema\": \"rurix.g41.water_wave_parity.v1\",\n  \
             \"gate\": \"g41.water.surface\",\n  \"subject\": \"g41_water_probe\",\n  \
             \"status\": \"{}\",\n  \"evidence_level\": \"measured_local\",\n  \
             \"frames\": {frames},\n  \"dim\": {WAVE_DIM},\n  \
             \"max_abs_diff\": {max_abs:e},\n  \"mismatched_cells\": {mismatched},\n  \
             \"worst_frame\": {worst_frame},\n  \"bit_equal\": {bit_equal},\n  \
             \"frozen_max_abs_diff\": {},\n  \"within_band\": {within_band},\n  \
             \"host_wave_digest\": \"sha256:{host_digest}\",\n  \
             \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"require_real\": {}}}\n}}\n",
            if within_band { "pass" } else { "fail" },
            frozen.map_or("null".to_owned(), |f| format!("{f:e}")),
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        );
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        std::fs::write(p, json).unwrap_or_else(|e| fail(&format!("写 evidence: {e}")));
        println!("{TAG}: evidence {}", p.display());
    }

    println!(
        "{TAG}: frames={frames} dim={WAVE_DIM} max_abs_diff={max_abs:e} \
         mismatched_cells={mismatched} worst_frame={worst_frame} \
         host_wave_digest=sha256:{host_digest}"
    );
    if within_band {
        println!(
            "{TAG}: PASS device↔host 波场对拍在冻结带内(bit_equal={bit_equal},frozen={})",
            frozen.map_or("freeze".to_owned(), |f| format!("{f:e}"))
        );
    } else {
        fail(&format!(
            "device↔host 波场超冻结带(max_abs_diff={max_abs:e} > frozen={},worst_frame={worst_frame})",
            frozen.map_or("n/a".to_owned(), |f| format!("{f:e}"))
        ));
    }
}
