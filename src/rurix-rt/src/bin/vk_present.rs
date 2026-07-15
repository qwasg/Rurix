//! mb1 W6 Vulkan win32 swapchain present 真跑 demo(RXS-0210 L4 present 落地;RFC-0011 §4.6)。
//!
//! 用法:`vk_present <vs.spv> <fs.spv>`(Phase 1 `rurixc --target vulkan vk_tri_vs.rx -o vs.spv`
//! / `vk_tri_fs.rx -o fs.spv` 产,方案 B 去 UserSemantic/SPV_GOOGLE)。在本机 Vulkan 设备
//! (NVIDIA / AMD 桌面)创建隐藏 win32 窗口 + `VkSurfaceKHR` + `VkSwapchainKHR`,渲染数帧居中
//! 三角形到 swapchain image → `vkCmdCopyImageToBuffer` 回读(反证 present 可数值校验)→ 转
//! `PRESENT_SRC_KHR` → `vkQueuePresentKHR`。逐像素断言(背景角 == clear 黑、中心被覆盖、覆盖
//! 计数 > 0)+ present 逐帧成功 + `VK_LAYER_KHRONOS_validation` 静默。
//!
//! 与 `vk_triangle`(offscreen)共享居中三角形几何 + 同一像素断言;差异 = present 通过真
//! swapchain 出图。win32 surface 为 Windows-only;非 Windows 平台 `run_graphics_present`
//! 返回确定性 `Err`(Android present = 尾门 G-MB1-7)。

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
            eprintln!("usage: vk_present <vs.spv> <fs.spv>");
            std::process::exit(2);
        }
    };
    let vs = read_spv(&vs_path);
    let fs = read_spv(&fs_path);

    const W: u32 = 64;
    const H: u32 = 64;
    const FRAMES: u32 = 3;
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

    let pixels = match rurix_rt::vk::run_graphics_present(
        &vs, &fs, &vertices, 32, &attrs, W, H, clear, FRAMES,
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("VK_PRESENT: run_graphics_present 失败: {e}");
            std::process::exit(1);
        }
    };

    let expected_len = (W * H * 4) as usize;
    if pixels.len() != expected_len {
        eprintln!(
            "VK_PRESENT: FAIL 回读长度 {} != {expected_len}(swapchain extent != {W}x{H}?)",
            pixels.len()
        );
        std::process::exit(1);
    }

    let px = |x: u32, y: u32| -> (u8, u8, u8, u8) {
        let o = ((y * W + x) * 4) as usize;
        (pixels[o], pixels[o + 1], pixels[o + 2], pixels[o + 3])
    };
    // 背景判定对通道序不敏感(黑=全零,任何 RGBA/BGRA 布局皆然)。
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
            "VK_PRESENT: FAIL 背景角 (0,{}) = {corner:?} != clear 黑(A=255)",
            H - 1
        );
        fail = true;
    }
    if is_background(center) {
        eprintln!(
            "VK_PRESENT: FAIL 中心 ({},{}) = {center:?} 仍是背景(未覆盖 / 光栅化未生效)",
            W / 2,
            H / 2
        );
        fail = true;
    }
    if covered == 0 {
        eprintln!("VK_PRESENT: FAIL 零像素被覆盖(光栅化未生效)");
        fail = true;
    }
    if fail {
        std::process::exit(1);
    }

    println!(
        "VK_PRESENT: ok W={W} H={H} frames={FRAMES} covered={covered} center=({},{},{})",
        center.0, center.1, center.2
    );
}
