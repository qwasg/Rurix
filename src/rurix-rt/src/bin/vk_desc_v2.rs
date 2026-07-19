//! G3.3 PR-S0 Vulkan graphics descriptor 建面 v2 最小闭环 demo(RXS-0230;RFC-0013 §4.B7)。
//!
//! `run_graphics_offscreen`(v1)与 `run_graphics_offscreen_v2`(v2,三类资源齐全:
//! 4×4→1×1 三层 mip 纹理〔逐层异色〕+ immutable sampler ×2〔clamp / wrap〕+ 8×8 RGBA32F
//! storage image)各渲染同一居中三角形(demo tri 着色器,无 descriptor 消费)→ 断言:
//! ① v2 三角形三断言(背景角 == clear / 中心覆盖非背景 / covered>0,镜像 vk_triangle);
//! ② v2 像素与 v1 **逐字节相等**(descriptor 建面对既有渲染输出零扰动 = 底座中性,
//!    MB1 语料零回归的运行时侧对照);
//! ③ `RURIX_VK_VALIDATION=1` 时 validation 零报错(fail-closed 由 run_graphics_* 内
//!    messenger 承担:sampler / mip 上传 / layout 迁移 / set-per-class 建面全程受校验)。
//!
//! 采样**数值判据**(≥6 模式,RFC-0013 §4.B8)须 descriptor-消费着色器语料,归步骤 63
//! PR-S3 主循环——本 demo 只见证 v2 descriptor 底座建面真跑,不冒充数值判据。

use rurix_rt::sampler::{Address, SamplerDesc};
use rurix_rt::vk::{
    GraphicsResource, StorageFormat, TextureData, demo_shaders_spv, run_graphics_offscreen,
    run_graphics_offscreen_v2,
};

/// 顶点属性格式 VK_FORMAT_R32G32B32A32_SFLOAT(Vulkan 枚举值;pos/color 各 vec4)。
const FORMAT_R32G32B32A32_SFLOAT: u32 = 109;

/// 追加一个 vec4<f32>(小端字节)到顶点缓冲。
fn push_vec4(buf: &mut Vec<u8>, v: [f32; 4]) {
    for f in v {
        buf.extend_from_slice(&f.to_le_bytes());
    }
}

/// SPIR-V 字节 → u32 字流(demo_shaders_spv 保证 len % 4 == 0)。
fn to_words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// 单色 RGBA8 层(w×h×4 字节)。
fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        v.extend_from_slice(&rgba);
    }
    v
}

fn main() {
    let (vs_b, fs_b, _saxpy) = demo_shaders_spv();
    if vs_b.is_empty() || fs_b.is_empty() {
        // build.rs codegen 降级(极少)→ 空切片,消费侧 SKIP(对齐既有降级纪律,非 fake)。
        println!("VK_DESC_V2: SKIP demo 着色器为空(build.rs codegen 降级)");
        return;
    }
    let vs = to_words(vs_b);
    let fs = to_words(fs_b);

    const W: u32 = 64;
    const H: u32 = 64;
    let clear = [0.0f32, 0.0, 0.0, 1.0]; // 背景黑(A=1)

    // 居中三角形(镜像 vk_triangle):每顶点 pos(vec4) @0 + color(vec4) @16,stride 32。
    let mut vertices: Vec<u8> = Vec::with_capacity(3 * 32);
    push_vec4(&mut vertices, [0.0, 0.7, 0.0, 1.0]); // v0 pos(上)
    push_vec4(&mut vertices, [1.0, 0.0, 0.0, 1.0]); // v0 color R
    push_vec4(&mut vertices, [-0.7, -0.7, 0.0, 1.0]); // v1 pos(左下)
    push_vec4(&mut vertices, [0.0, 1.0, 0.0, 1.0]); // v1 color G
    push_vec4(&mut vertices, [0.7, -0.7, 0.0, 1.0]); // v2 pos(右下)
    push_vec4(&mut vertices, [0.0, 0.0, 1.0, 1.0]); // v2 color B
    let attrs = [
        (0u32, FORMAT_R32G32B32A32_SFLOAT, 0u32),
        (1u32, FORMAT_R32G32B32A32_SFLOAT, 16u32),
    ];

    // ── v1 基线 ──
    let v1 = match run_graphics_offscreen(&vs, &fs, &vertices, 32, &attrs, W, H, clear) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("VK_DESC_V2: v1 run_graphics_offscreen 失败: {e}");
            std::process::exit(1);
        }
    };

    // ── v2:三类资源齐全(SRV mip 纹理 + Sampler ×2 + UAV storage)──
    let resources = [
        GraphicsResource::Texture2D {
            width: 4,
            height: 4,
            data: TextureData::Rgba8(vec![
                solid(4, 4, [255, 0, 0, 255]), // level 0 红
                solid(2, 2, [0, 255, 0, 255]), // level 1 绿
                solid(1, 1, [0, 0, 255, 255]), // level 2 蓝(逐层异色,mip 链完整)
            ]),
        },
        GraphicsResource::Sampler(SamplerDesc::default()), // clamp
        GraphicsResource::Sampler(SamplerDesc {
            address: Address::Wrap,
            ..SamplerDesc::default()
        }), // wrap(与 clamp 并存 = Sampler 轴 binding 0/1)
        GraphicsResource::StorageImage {
            width: 8,
            height: 8,
            format: StorageFormat::Rgba32Float,
        },
    ];
    let v2 =
        match run_graphics_offscreen_v2(&vs, &fs, &vertices, 32, &attrs, W, H, clear, &resources) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("VK_DESC_V2: v2 run_graphics_offscreen_v2 失败: {e}");
                std::process::exit(1);
            }
        };

    // ── 断言 ──
    let expected_len = (W * H * 4) as usize;
    let mut fail = false;
    if v2.len() != expected_len {
        eprintln!(
            "VK_DESC_V2: FAIL v2 回读长度 {} != {expected_len}",
            v2.len()
        );
        fail = true;
    }
    let px = |p: &[u8], x: u32, y: u32| -> (u8, u8, u8, u8) {
        let o = ((y * W + x) * 4) as usize;
        (p[o], p[o + 1], p[o + 2], p[o + 3])
    };
    let is_background = |p: (u8, u8, u8, u8)| p.0 == 0 && p.1 == 0 && p.2 == 0;
    let mut covered = 0usize;
    for y in 0..H {
        for x in 0..W {
            if !is_background(px(&v2, x, y)) {
                covered += 1;
            }
        }
    }
    let corner = px(&v2, 0, H - 1);
    let center = px(&v2, W / 2, H / 2);
    if !is_background(corner) || corner.3 != 255 {
        eprintln!("VK_DESC_V2: FAIL v2 背景角 = {corner:?} != clear 黑(A=255)");
        fail = true;
    }
    if is_background(center) {
        eprintln!("VK_DESC_V2: FAIL v2 中心 = {center:?} 仍是背景(未覆盖)");
        fail = true;
    }
    if covered == 0 {
        eprintln!("VK_DESC_V2: FAIL v2 零像素被覆盖");
        fail = true;
    }
    // 底座中性:descriptor 建面(上传 / 迁移 / 绑定)不得扰动既有渲染输出。
    if v1 != v2 {
        let diff = v1.iter().zip(&v2).filter(|(a, b)| a != b).count();
        eprintln!("VK_DESC_V2: FAIL v1/v2 像素不等(diff 字节 = {diff};底座应中性)");
        fail = true;
    }
    if fail {
        std::process::exit(1);
    }

    println!(
        "VK_DESC_V2: ok W={W} H={H} covered={covered} v1v2_equal=true resources=4(srv-mip3+sampler2+uav)"
    );
}
