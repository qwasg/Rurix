//! G31+ 波 C Task C16:M61 ③ mesh shader HW 路径 vs 现 VS 光栅路径 **measured 对照**
//! harness（RFC-0034 重判表三项闭集之③;TODO §3.1 #24;门
//! `g31.waveC.meshbench` 由 ci/g31_mesh_vs_raster_bench.py 承载）。
//!
//! ## 用法
//! `g31_mesh_vs_raster_bench <mesh.spv> <vs_fetch.spv> <vs_proc.spv> <fs.spv>
//!   [--triangles N] [--frames K] [--warmup W]`
//! SPV 由 CI 驱动经 glslangValidator 现编（GLSL 源内嵌 ci/g31_mesh_vs_raster_bench
//! .py;host 侧顶点生成与 GLSL 逐字同源 = 本文件 `tri_vert_ndc`）。
//!
//! ## 判据承载（measured,不设通过线——G6 无硬门纪律）
//! 三臂同一确定性三角形集（u32 PCG 整数哈希;`precise` 禁 FMA）⇒ 像素 digest
//! 逐臂对拍（零差 = 同一几何真上屏结构证据）+ GPU timestamp 逐帧 GPU ms 主口径
//! （median/mean/min/max）+ 壁钟副口径,如实登记。①vs_fetch（device-local VB
//! 取数 = 现光栅路径形态）vs ③mesh_procedural = 主对照;②vs_procedural = 隔离
//! 取数成本的解释性臂。
//!
//! **device 真跑 / SKIP 三态**:无 Vulkan loader / `meshShader` feature 缺失 →
//! 确定性 `Err` → `MESH_BENCH: SKIP` 退 0（dev-env degrade,非 fake pass）;
//! `RURIX_REQUIRE_REAL=1` 翻硬红。`RURIX_VK_VALIDATION=1` 校验 ERROR → FAIL。

const GRID_W: u32 = 240;
const GRID_H: u32 = 135;
const CELL_PX: u32 = 8;
const WIDTH: u32 = GRID_W * CELL_PX; // 1920
const HEIGHT: u32 = GRID_H * CELL_PX; // 1080

const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "graphics queue",
    "vkCreateInstance",
    "mesh shader feature",
];

/// PCG RXS-M-XS u32 哈希——与 GLSL 源逐字同源（GLSL 内嵌
/// ci/g31_mesh_vs_raster_bench.py;任一侧改动 = 对拍面红）。
fn pcg_hash(v: u32) -> u32 {
    let state = v.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// 三角形 slot 顶点 NDC 坐标（与 GLSL tri_vert 逐字同源;同序 IEEE fp32 无 FMA）。
fn tri_vert_ndc(tri: u32, slot: u32) -> [f32; 4] {
    let r0 = pcg_hash(tri);
    let r1 = pcg_hash(tri ^ 0x9E37_79B9);
    let cx = r0 % GRID_W;
    let cy = r1 % GRID_H;
    let ox = (r0 >> 8) & 7;
    let oy = (r1 >> 8) & 7;
    let flip = (r0 >> 16) & 1;
    let bx = (cx * CELL_PX + ox) as f32;
    let by = (cy * CELL_PX + oy) as f32;
    let s = CELL_PX as f32;
    let (px, py) = if flip == 0 {
        match slot {
            0 => (bx, by),
            1 => (bx + s, by),
            _ => (bx, by + s),
        }
    } else {
        match slot {
            0 => (bx + s, by),
            1 => (bx + s, by + s),
            _ => (bx, by + s),
        }
    };
    let w = (GRID_W * CELL_PX) as f32;
    let h = (GRID_H * CELL_PX) as f32;
    let nx = (px / w) * 2.0 - 1.0;
    let ny = (py / h) * 2.0 - 1.0;
    [nx, ny, 0.0, 1.0]
}

fn read_spv(path: &str) -> Vec<u32> {
    let raw = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("读 {path} 失败: {e}");
        std::process::exit(2);
    });
    if !raw.len().is_multiple_of(4) {
        eprintln!("{path}: SPIR-V 字节须 4 字节对齐");
        std::process::exit(2);
    }
    raw.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

fn stats(samples: &[f64]) -> (f64, f64, f64, f64) {
    let mut s = samples.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = s.len();
    let median = if n == 0 {
        f64::NAN
    } else if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    };
    let mean = if n == 0 {
        f64::NAN
    } else {
        s.iter().sum::<f64>() / n as f64
    };
    let min = if n == 0 { f64::NAN } else { s[0] };
    let max = if n == 0 { f64::NAN } else { s[n - 1] };
    (median, mean, min, max)
}

fn fnum(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.6}")
    } else {
        "null".into()
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (mesh_p, vsf_p, vsp_p, fs_p) = match (args.next(), args.next(), args.next(), args.next()) {
        (Some(a), Some(b), Some(c), Some(d)) => (a, b, c, d),
        _ => {
            eprintln!(
                "usage: g31_mesh_vs_raster_bench <mesh.spv> <vs_fetch.spv> <vs_proc.spv> <fs.spv> \
                 [--triangles N] [--frames K] [--warmup W]"
            );
            std::process::exit(2);
        }
    };
    let mut triangles: u32 = 262_144;
    let mut frames: u32 = 60;
    let mut warmup: u32 = 10;
    let rest: Vec<String> = args.collect();
    let mut i = 0;
    while i + 1 < rest.len() {
        match rest[i].as_str() {
            "--triangles" => triangles = rest[i + 1].parse().unwrap_or(triangles),
            "--frames" => frames = rest[i + 1].parse().unwrap_or(frames),
            "--warmup" => warmup = rest[i + 1].parse().unwrap_or(warmup),
            _ => {}
        }
        i += 2;
    }
    if triangles % 64 != 0 || triangles == 0 || frames == 0 {
        eprintln!("--triangles 须为 64 非零整数倍,--frames 须 > 0");
        std::process::exit(2);
    }

    let mesh = read_spv(&mesh_p);
    let vs_fetch = read_spv(&vsf_p);
    let vs_proc = read_spv(&vsp_p);
    let fs = read_spv(&fs_p);

    // host 侧 vertex buffer 填充（vs_fetch 臂取数面;与 GLSL/host 哈希逐字同源）。
    let mut tri_verts: Vec<[f32; 4]> = Vec::with_capacity(triangles as usize * 3);
    for tri in 0..triangles {
        for slot in 0..3 {
            tri_verts.push(tri_vert_ndc(tri, slot));
        }
    }

    println!(
        "[g31_mesh_vs_raster_bench] M61 ③ mesh HW vs VS 光栅 measured 对照 \
         ({WIDTH}x{HEIGHT} N={triangles} frames={frames} warmup={warmup})"
    );
    let rep = match rurix_rt::vk::run_mesh_vs_raster_bench(
        &mesh, &vs_fetch, &vs_proc, &fs, WIDTH, HEIGHT, GRID_W, GRID_H, CELL_PX, &tri_verts, 64,
        frames, warmup,
    ) {
        Ok(r) => r,
        Err(e) if is_no_device(&e) => {
            println!(
                "MESH_BENCH: SKIP 无 Vulkan 设备 / mesh feature 缺失({})",
                e.trim()
            );
            return;
        }
        Err(e) => {
            eprintln!("MESH_BENCH: FAIL run_mesh_vs_raster_bench: {e}");
            std::process::exit(1);
        }
    };

    let mut arms_json: Vec<String> = Vec::new();
    let mut digests: Vec<String> = Vec::new();
    for arm in &rep.arms {
        let (gmed, gmean, gmin, gmax) = stats(&arm.gpu_ms_samples);
        let (wmed, wmean, _wmin, _wmax) = stats(&arm.wall_ms_samples);
        let digest = rurix_pkg::sha256::hex_digest(&arm.pixels);
        digests.push(digest.clone());
        arms_json.push(format!(
            "{{\"arm\":\"{}\",\"gpu_ms_median\":{},\"gpu_ms_mean\":{},\"gpu_ms_min\":{},\
             \"gpu_ms_max\":{},\"wall_ms_median\":{},\"wall_ms_mean\":{},\"samples\":{},\
             \"pixel_digest\":\"sha256:{}\"}}",
            arm.arm,
            fnum(gmed),
            fnum(gmean),
            fnum(gmin),
            fnum(gmax),
            fnum(wmed),
            fnum(wmean),
            arm.gpu_ms_samples.len(),
            digest,
        ));
    }
    let digest_all_equal =
        digests.len() == 3 && digests[0] == digests[1] && digests[1] == digests[2];
    println!(
        "MESH_BENCH_JSON: {{\"schema\":\"rurix.g31.mesh_vs_raster_bench.v1\",\
         \"subject\":\"g31_mesh_vs_raster_bench\",\"width\":{},\"height\":{},\"triangles\":{},\
         \"frames\":{},\"warmup\":{},\"timestamp_period_ns\":{},\"device_name\":\"{}\",\
         \"driver_version\":{},\"vendor_id\":{},\"api_version\":{},\"arms\":[{}],\
         \"digest_all_equal\":{}}}",
        rep.width,
        rep.height,
        rep.triangles,
        rep.frames,
        rep.warmup,
        rep.timestamp_period_ns,
        rep.device_name.replace('"', "'"),
        rep.driver_version,
        rep.vendor_id,
        rep.api_version,
        arms_json.join(","),
        digest_all_equal,
    );
    if digest_all_equal {
        println!(
            "MESH_BENCH: PASS 三臂像素 digest 位级全等（同一三角形集真上屏;sha256:{}…）",
            &digests[0][..16]
        );
    } else {
        // 终图分叉 = 对拍面破坏（几何不一致/编译器数值漂移）——诚实 FAIL,不伪造。
        println!("MESH_BENCH: FAIL 三臂像素 digest 未全等: {digests:?}");
        std::process::exit(1);
    }
}
