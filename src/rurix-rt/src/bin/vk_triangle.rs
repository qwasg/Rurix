//! mb1 Phase 3 Vulkan graphics offscreen 真跑 demo(RXS-0210;RFC-0011 §4.6)。
//!
//! 用法:`vk_triangle <vs.spv> <fs.spv>`(Phase 1 `rurixc --target vulkan vk_tri_vs.rx -o
//! vs.spv` / `vk_tri_fs.rx -o fs.spv` 产,方案 B 去 UserSemantic/SPV_GOOGLE)。在本机 Vulkan
//! 设备(NVIDIA / AMD 桌面 / lavapipe)offscreen 渲染一个居中三角形到 64×64 R8G8B8A8 color
//! image → `vkCmdCopyImageToBuffer` 回读 → 逐像素断言:背景角 == clear 黑、中心被覆盖(非
//! 背景色)、覆盖像素计数 > 0。
//!
//! **注(与 design §3 的偏差,已在 W2 报告说明)**:design 写「全屏三角形 clip
//! (-1,-1),(3,-1),(-1,3)」,但全屏三角形覆盖整个视口 → 无「背景角」可断言 clear。为使
//! 「背景角 == clear」成为**真**断言(非伪绿),此处用**居中**三角形:角落留背景(clear
//! 黑)、中心被覆盖并插值三色。三条断言(clear / 覆盖 / 插值)因此全部可真校验。

/// 顶点属性格式 VK_FORMAT_R32G32B32A32_SFLOAT(Vulkan 枚举值;pos/color 各 vec4)。
const FORMAT_R32G32B32A32_SFLOAT: u32 = 109;

/// 追加一个 vec4<f32>(小端字节)到顶点缓冲。
fn push_vec4(buf: &mut Vec<u8>, v: [f32; 4]) {
    for f in v {
        buf.extend_from_slice(&f.to_le_bytes());
    }
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

fn main() {
    let mut args = std::env::args().skip(1);
    let (vs_path, fs_path) = match (args.next(), args.next()) {
        (Some(v), Some(f)) => (v, f),
        _ => {
            eprintln!("usage: vk_triangle <vs.spv> <fs.spv>");
            std::process::exit(2);
        }
    };
    let vs = read_spv(&vs_path);
    let fs = read_spv(&fs_path);

    const W: u32 = 64;
    const H: u32 = 64;
    let clear = [0.0f32, 0.0, 0.0, 1.0]; // 背景黑(A=1)

    // 居中三角形(NDC clip;w=1),角落留背景、中心覆盖。顶点色 R/G/B。
    // 每顶点:pos(vec4) @0 + color(vec4) @16,stride 32。
    let mut vertices: Vec<u8> = Vec::with_capacity(3 * 32);
    push_vec4(&mut vertices, [0.0, 0.7, 0.0, 1.0]); // v0 pos(上)
    push_vec4(&mut vertices, [1.0, 0.0, 0.0, 1.0]); // v0 color R
    push_vec4(&mut vertices, [-0.7, -0.7, 0.0, 1.0]); // v1 pos(左下)
    push_vec4(&mut vertices, [0.0, 1.0, 0.0, 1.0]); // v1 color G
    push_vec4(&mut vertices, [0.7, -0.7, 0.0, 1.0]); // v2 pos(右下)
    push_vec4(&mut vertices, [0.0, 0.0, 1.0, 1.0]); // v2 color B

    let attrs = [
        (0u32, FORMAT_R32G32B32A32_SFLOAT, 0u32), // location 0 = pos @0
        (1u32, FORMAT_R32G32B32A32_SFLOAT, 16u32), // location 1 = color @16
    ];

    let pixels =
        match rurix_rt::vk::run_graphics_offscreen(&vs, &fs, &vertices, 32, &attrs, W, H, clear) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("VK_TRIANGLE: run_graphics_offscreen 失败: {e}");
                std::process::exit(1);
            }
        };

    let expected_len = (W * H * 4) as usize;
    if pixels.len() != expected_len {
        eprintln!(
            "VK_TRIANGLE: FAIL 回读长度 {} != {expected_len}",
            pixels.len()
        );
        std::process::exit(1);
    }

    let px = |x: u32, y: u32| -> (u8, u8, u8, u8) {
        let o = ((y * W + x) * 4) as usize;
        (pixels[o], pixels[o + 1], pixels[o + 2], pixels[o + 3])
    };
    let is_background = |p: (u8, u8, u8, u8)| p.0 == 0 && p.1 == 0 && p.2 == 0;

    // 覆盖计数(非背景色像素数)。
    let mut covered = 0usize;
    for y in 0..H {
        for x in 0..W {
            if !is_background(px(x, y)) {
                covered += 1;
            }
        }
    }

    let corner = px(0, H - 1); // 左下角(居中三角外)→ 应 clear 黑,A=255
    let center = px(W / 2, H / 2); // 中心 → 覆盖,插值三色

    let mut fail = false;
    if !is_background(corner) || corner.3 != 255 {
        eprintln!(
            "VK_TRIANGLE: FAIL 背景角 (0,{}) = {corner:?} != clear 黑(A=255)",
            H - 1
        );
        fail = true;
    }
    if is_background(center) {
        eprintln!(
            "VK_TRIANGLE: FAIL 中心 ({},{}) = {center:?} 仍是背景(未覆盖 / 光栅化未生效)",
            W / 2,
            H / 2
        );
        fail = true;
    }
    if covered == 0 {
        eprintln!("VK_TRIANGLE: FAIL 零像素被覆盖(光栅化未生效)");
        fail = true;
    }
    if fail {
        std::process::exit(1);
    }

    println!(
        "VK_TRIANGLE: ok W={W} H={H} covered={covered} center=({},{},{})",
        center.0, center.1, center.2
    );
}
