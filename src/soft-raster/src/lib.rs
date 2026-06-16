//! soft-raster — Rurix G0 软光栅 host CPU 参考实现(M7.3,D-M7-3)。
//!
//! 条款:spec/softraster.md RXS-0118 ~ RXS-0121(图元分桶到 tile binning / tile 光栅
//! 覆盖判定·重心坐标·边函数 / 深度 z-buffer 写入与深度测试 / tonemap HDR→LDR 像素量化)。
//!
//! 纪律:**全 safe**(`unsafe_code = "deny"`,继承 workspace lints);纯函数、确定性
//! ——固定输入 → 逐字节确定帧像素。与 device kernel(`src/rurix-rt/kernels/sr_*.rx`)
//! 标量数值语义同义(05 §1 device ⊂ host);分桶遍历序 / 深度合成序固定,每像素/桶
//! 单一 owner 写入,atomics-free。复用 [`image_io`] 的 `Rgb` / `ImageBuffer` / PPM P6
//! 确定编码(RXS-0114~0117)与 `f32→u8` 确定量化(RXS-0116),产逐字节确定帧,为
//! G-M7-3 safe 覆盖与确定性帧像素门铺底。

use image_io::{ImageBuffer, Rgb};

/// 帧宽(像素)。
pub const WIDTH: u32 = 32;
/// 帧高(像素)。
pub const HEIGHT: u32 = 24;
/// 方形 tile 边长(像素)。
pub const TILE_SIZE: u32 = 8;
/// tile 网格列数。
pub const TILES_X: u32 = WIDTH / TILE_SIZE;
/// tile 网格行数。
pub const TILES_Y: u32 = HEIGHT / TILE_SIZE;
/// 远平面深度哨兵(z-buffer 初值;RXS-0120)。
pub const Z_FAR: f32 = f32::INFINITY;

/// 屏幕空间顶点(像素 xy + 深度 z + 颜色 rgb,分量 `f32`)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// 屏幕 x(像素)。
    pub x: f32,
    /// 屏幕 y(像素)。
    pub y: f32,
    /// 深度(less 测试,越小越近)。
    pub z: f32,
    /// 红分量(HDR `f32`)。
    pub r: f32,
    /// 绿分量(HDR `f32`)。
    pub g: f32,
    /// 蓝分量(HDR `f32`)。
    pub b: f32,
}

/// 三角形(三顶点)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tri {
    /// 三顶点(逆时针约定,RXS-0119 覆盖判定)。
    pub v: [Vertex; 3],
}

/// 边函数(二维叉积,RXS-0119):`edge(A,B,P) = (Bx-Ax)*(Py-Ay) - (By-Ay)*(Px-Ax)`。
///
/// 与 device `edge`(`sr_raster_tile.rx`)标量数值语义同义。
#[must_use]
pub fn edge(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

/// 三角形屏幕包围盒 `(min_x, max_x, min_y, max_y)`(RXS-0118 binning 用)。
#[must_use]
pub fn tri_bbox(tri: &Tri) -> (f32, f32, f32, f32) {
    let [a, b, c] = tri.v;
    (
        a.x.min(b.x).min(c.x),
        a.x.max(b.x).max(c.x),
        a.y.min(b.y).min(c.y),
        a.y.max(b.y).max(c.y),
    )
}

/// 半开区间相交(确定性,RXS-0118):`a0 < b1 && b0 < a1`。
#[must_use]
fn overlaps(a0: f32, a1: f32, b0: f32, b1: f32) -> bool {
    a0 < b1 && b0 < a1
}

/// 图元分桶到 tile(RXS-0118):按图元下标升序遍历,覆盖本 tile 像素矩形的图元下标
/// 依序追加入本桶。返回 `tiles_x * tiles_y` 个桶(行主序 `ty * tiles_x + tx`),每桶
/// 为覆盖图元下标的确定性升序列表。每桶单一 owner 装配,atomics-free、确定性遍历序。
#[must_use]
pub fn bin_triangles(tris: &[Tri]) -> Vec<Vec<usize>> {
    let ntiles = (TILES_X * TILES_Y) as usize;
    let mut bins: Vec<Vec<usize>> = vec![Vec::new(); ntiles];
    for (tile, bin) in bins.iter_mut().enumerate() {
        let tx = (tile as u32) % TILES_X;
        let ty = (tile as u32) / TILES_X;
        let tile_x0 = (tx * TILE_SIZE) as f32;
        let tile_x1 = ((tx + 1) * TILE_SIZE) as f32;
        let tile_y0 = (ty * TILE_SIZE) as f32;
        let tile_y1 = ((ty + 1) * TILE_SIZE) as f32;
        for (k, tri) in tris.iter().enumerate() {
            let (bx0, bx1, by0, by1) = tri_bbox(tri);
            if overlaps(bx0, bx1, tile_x0, tile_x1) && overlaps(by0, by1, tile_y0, tile_y1) {
                bin.push(k);
            }
        }
    }
    bins
}

/// 像素的覆盖 / 插值结果(RXS-0119):覆盖时携重心插值的深度与颜色。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fragment {
    /// 插值深度(less 测试输入,RXS-0120)。
    pub z: f32,
    /// 插值颜色(HDR `f32`,通道 R,G,B)。
    pub rgb: [f32; 3],
}

/// 对像素中心 `(px, py)` 求三角形覆盖与重心插值(RXS-0119)。
///
/// 三边函数同号(`≥ 0`,逆时针约定)且二倍面积非零方覆盖;退化三角形(`area2 == 0`)
/// 不覆盖(确定性,不除零)。覆盖时以重心权重插值深度与颜色。
#[must_use]
pub fn shade_pixel(tri: &Tri, px: f32, py: f32) -> Option<Fragment> {
    let [v0, v1, v2] = tri.v;
    let area2 = edge(v0.x, v0.y, v1.x, v1.y, v2.x, v2.y);
    if area2 == 0.0 {
        return None;
    }
    let e0 = edge(v1.x, v1.y, v2.x, v2.y, px, py);
    let e1 = edge(v2.x, v2.y, v0.x, v0.y, px, py);
    let e2 = edge(v0.x, v0.y, v1.x, v1.y, px, py);
    if e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0 {
        let w0 = e0 / area2;
        let w1 = e1 / area2;
        let w2 = e2 / area2;
        let z = w0 * v0.z + w1 * v1.z + w2 * v2.z;
        let r = w0 * v0.r + w1 * v1.r + w2 * v2.r;
        let g = w0 * v0.g + w1 * v1.g + w2 * v2.g;
        let b = w0 * v0.b + w1 * v1.b + w2 * v2.b;
        Some(Fragment { z, rgb: [r, g, b] })
    } else {
        None
    }
}

/// 深度测试 less 约定(RXS-0120):候选深度 `z_cand` 通过当且仅当 `z_cand < z_buf`。
/// 相等不覆盖(保留先到者),保确定性合成序。
#[must_use]
pub fn depth_test_less(z_cand: f32, z_buf: f32) -> bool {
    z_cand < z_buf
}

/// 渲染一帧(RXS-0118~0120 管线):binning → 逐像素覆盖/重心 → less 深度合成。
///
/// 返回 HDR 颜色帧缓冲(行主序 `[f32;3]`)。每像素 owner 按桶内图元升序(分桶序)
/// 串行做 less 深度测试,确定性、atomics-free。
#[must_use]
pub fn render_hdr(tris: &[Tri]) -> Vec<[f32; 3]> {
    let w = WIDTH as usize;
    let h = HEIGHT as usize;
    let mut color = vec![[0.0f32, 0.0, 0.0]; w * h];
    let mut zbuf = vec![Z_FAR; w * h];
    let bins = bin_triangles(tris);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let tx = x / TILE_SIZE;
            let ty = y / TILE_SIZE;
            let tile = (ty * TILES_X + tx) as usize;
            let idx = (y * WIDTH + x) as usize;
            let px = (x as f32) + 0.5;
            let py = (y as f32) + 0.5;
            // 桶内图元升序(RXS-0118 分桶序)→ 固定深度合成序(RXS-0120)。
            for &k in &bins[tile] {
                if let Some(frag) = shade_pixel(&tris[k], px, py)
                    && depth_test_less(frag.z, zbuf[idx])
                {
                    zbuf[idx] = frag.z;
                    color[idx] = frag.rgb;
                }
            }
        }
    }
    color
}

/// `f32` 分量 → `u8` 确定量化(RXS-0121,口径对接 image-io RXS-0116):钳制 `[0,1]`
/// (NaN→0)后就近取整 `floor(clamp(c)*255+0.5)`(半值向上)。与 device `sr_quantize`
/// (`sr_tonemap.rx`)及 image-io 编码量化数值同义。
#[must_use]
pub fn tonemap_channel(c: f32) -> u8 {
    let clamped = if c.is_nan() { 0.0 } else { c.clamp(0.0, 1.0) };
    (clamped * 255.0 + 0.5).floor() as u8
}

/// tonemap HDR 帧缓冲 → image-io `ImageBuffer<Rgb>`(RXS-0121)。
///
/// 像素 HDR 颜色直接承载为 `Rgb`(`f32`),量化由 image-io 编码(RXS-0116)在落盘时
/// 施加;两路径 `f32→u8` 量化数值同义(见 [`tonemap_channel`])。
#[must_use]
pub fn tonemap_frame(hdr: &[[f32; 3]]) -> ImageBuffer<Rgb> {
    let mut buf = ImageBuffer::new(WIDTH, HEIGHT, Rgb::new(0.0, 0.0, 0.0));
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let idx = (y * WIDTH + x) as usize;
            let [r, g, b] = hdr[idx];
            buf.set(x, y, Rgb::new(r, g, b));
        }
    }
    buf
}

/// 固定场景三角形(确定性;`frame` 决定一个确定性平移,产动画序列)。
///
/// 两个互相遮挡的三角形(不同深度 / 颜色),验证 binning + 覆盖 + 重心 + less 深度。
#[must_use]
pub fn fixed_scene(frame: u32) -> Vec<Tri> {
    // 帧间确定性平移(整数像素,纯 f32),无随机量 / 时间戳。
    let dx = (frame as f32) * 2.0;
    // 近三角形(z=0.2,暖色),覆盖左中区域。
    let near = Tri {
        v: [
            Vertex {
                x: 4.0 + dx,
                y: 3.0,
                z: 0.2,
                r: 0.9,
                g: 0.3,
                b: 0.1,
            },
            Vertex {
                x: 22.0 + dx,
                y: 6.0,
                z: 0.2,
                r: 0.9,
                g: 0.3,
                b: 0.1,
            },
            Vertex {
                x: 7.0 + dx,
                y: 20.0,
                z: 0.2,
                r: 0.9,
                g: 0.3,
                b: 0.1,
            },
        ],
    };
    // 远三角形(z=0.8,冷色),与近三角形部分重叠 → 被遮挡区域应取近三角形颜色。
    let far = Tri {
        v: [
            Vertex {
                x: 10.0,
                y: 2.0,
                z: 0.8,
                r: 0.1,
                g: 0.4,
                b: 0.9,
            },
            Vertex {
                x: 28.0,
                y: 10.0,
                z: 0.8,
                r: 0.1,
                g: 0.4,
                b: 0.9,
            },
            Vertex {
                x: 14.0,
                y: 22.0,
                z: 0.8,
                r: 0.1,
                g: 0.4,
                b: 0.9,
            },
        ],
    };
    vec![near, far]
}

/// 渲染确定性帧序列(RXS-0118~0121 完整管线 → tonemap 帧)。
#[must_use]
pub fn render_sequence(frames: u32) -> Vec<ImageBuffer<Rgb>> {
    (0..frames)
        .map(|f| {
            let scene = fixed_scene(f);
            let hdr = render_hdr(&scene);
            tonemap_frame(&hdr)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image_io::{ImageFormat, encode};

    //@ spec: RXS-0118
    // 图元分桶到 tile:覆盖 tile 的图元入桶、不覆盖不入桶;桶内升序确定性。
    #[test]
    fn binning_assigns_overlapping_tiles_deterministically() {
        // 单三角形仅覆盖左上 tile 区域 [0,8)x[0,8)。
        let tri = Tri {
            v: [
                Vertex {
                    x: 1.0,
                    y: 1.0,
                    z: 0.5,
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                },
                Vertex {
                    x: 6.0,
                    y: 1.0,
                    z: 0.5,
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                },
                Vertex {
                    x: 1.0,
                    y: 6.0,
                    z: 0.5,
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                },
            ],
        };
        let bins = bin_triangles(&[tri]);
        // 左上 tile (0,0) 命中,其余不命中。
        assert_eq!(bins[0], vec![0]);
        assert!(bins[1].is_empty());
        // 确定性:重复分桶结果一致。
        assert_eq!(bin_triangles(&[tri]), bins);

        // 跨多个 tile 的大三角形:桶内图元下标升序。
        let scene = fixed_scene(0);
        let bins2 = bin_triangles(&scene);
        for bin in &bins2 {
            let mut sorted = bin.clone();
            sorted.sort_unstable();
            assert_eq!(*bin, sorted, "桶内图元下标须为确定性升序");
        }
    }

    //@ spec: RXS-0119
    // tile 光栅:边函数符号、覆盖判定、重心插值、退化三角形不覆盖。
    #[test]
    fn rasterize_edge_coverage_barycentric() {
        let tri = Tri {
            v: [
                Vertex {
                    x: 0.0,
                    y: 0.0,
                    z: 0.5,
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                },
                Vertex {
                    x: 10.0,
                    y: 0.0,
                    z: 0.5,
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                },
                Vertex {
                    x: 0.0,
                    y: 10.0,
                    z: 0.5,
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                },
            ],
        };
        // 边函数符号:逆时针三角形对内部点三边均 >= 0。
        let inside = shade_pixel(&tri, 1.5, 1.5);
        assert!(inside.is_some());
        // 重心插值:三顶点权重和为 1 → 颜色分量和为 1。
        let f = inside.unwrap();
        let sum = f.rgb[0] + f.rgb[1] + f.rgb[2];
        assert!((sum - 1.0).abs() < 1e-5, "重心权重和应为 1,实得 {sum}");
        // 外部点不覆盖。
        assert!(shade_pixel(&tri, 9.0, 9.0).is_none());
        // 退化三角形(共线 → area2==0)不覆盖。
        let degen = Tri {
            v: [
                Vertex {
                    x: 0.0,
                    y: 0.0,
                    z: 0.5,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                },
                Vertex {
                    x: 4.0,
                    y: 0.0,
                    z: 0.5,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                },
                Vertex {
                    x: 8.0,
                    y: 0.0,
                    z: 0.5,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                },
            ],
        };
        assert!(shade_pixel(&degen, 2.0, 0.0).is_none());
        // 边函数二维叉积符号:edge 在左侧为正。
        assert!(edge(0.0, 0.0, 10.0, 0.0, 0.0, 5.0) > 0.0);
    }

    //@ spec: RXS-0120
    // 深度:less 测试、遮挡(近覆盖远)、相等不覆盖、固定合成序确定性。
    #[test]
    fn depth_less_test_and_occlusion() {
        assert!(depth_test_less(0.2, 0.8));
        assert!(!depth_test_less(0.8, 0.2));
        // 相等不覆盖(保留先到者)。
        assert!(!depth_test_less(0.5, 0.5));

        // 近三角形(z=0.2)遮挡远三角形(z=0.8):重叠像素取近三角形暖色。
        let scene = fixed_scene(0);
        let hdr = render_hdr(&scene);
        // 找到一个两三角形都覆盖的像素(取近色 r>g 暖色)。
        let mut found = false;
        for (idx, px) in hdr.iter().enumerate() {
            let x = (idx as u32) % WIDTH;
            let y = (idx as u32) / WIDTH;
            let cx = (x as f32) + 0.5;
            let cy = (y as f32) + 0.5;
            let near_cov = shade_pixel(&scene[0], cx, cy).is_some();
            let far_cov = shade_pixel(&scene[1], cx, cy).is_some();
            if near_cov && far_cov {
                // 近三角形暖色 r=0.9 > b=0.1。
                assert!(px[0] > px[2], "遮挡像素应取近三角形暖色");
                found = true;
                break;
            }
        }
        assert!(found, "场景应存在两三角形重叠的遮挡像素");
        // 固定输入两次渲染逐值一致(确定性合成序)。
        assert_eq!(render_hdr(&fixed_scene(0)), hdr);
    }

    //@ spec: RXS-0121
    // tonemap:量化边界 0/255/NaN/半值;帧 → PPM 字节确定性,两次编码逐字节一致。
    #[test]
    fn tonemap_quantize_and_frame_determinism() {
        // 量化边界(对接 imageio RXS-0116:clamp+NaN→0+半值向上)。
        assert_eq!(tonemap_channel(0.0), 0);
        assert_eq!(tonemap_channel(1.0), 255);
        assert_eq!(tonemap_channel(-1.0), 0);
        assert_eq!(tonemap_channel(2.0), 255);
        assert_eq!(tonemap_channel(f32::NAN), 0);
        assert_eq!(tonemap_channel(0.5), 128);

        // 帧确定性:同输入两次渲染 → PPM 编码逐字节一致。
        let seq_a = render_sequence(3);
        let seq_b = render_sequence(3);
        for (a, b) in seq_a.iter().zip(seq_b.iter()) {
            let ba = encode(a, ImageFormat::Ppm).unwrap();
            let bb = encode(b, ImageFormat::Ppm).unwrap();
            assert_eq!(ba, bb, "同输入两次编码应逐字节一致(确定性)");
            assert!(!ba.is_empty());
        }
    }
}
