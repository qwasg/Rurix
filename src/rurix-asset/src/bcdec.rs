//! 独立 BC7 mode-6 解码器(仅解码;供 M83 tolerance 对拍,避免 vendor 自证)。
//!
//! 首版只覆盖本 crate 过渡编码器产出的 mode-6 块;其它 mode → 透明黑。

/// 解码 BC7 块字节为 RGBA8(行主序)。`blocks` 长度须 = ceil(w/4)*ceil(h/4)*16。
pub fn decode_bc7_rgba8(blocks: &[u8], width: u32, height: u32) -> Vec<u8> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];
    let mut bi = 0usize;
    for by in 0..bh {
        for bx in 0..bw {
            if bi + 16 > blocks.len() {
                return out;
            }
            let mut texels = [[0u8; 4]; 16];
            decode_bc7_block(&blocks[bi..bi + 16], &mut texels);
            bi += 16;
            for ty in 0..4u32 {
                for tx in 0..4u32 {
                    let x = bx * 4 + tx;
                    let y = by * 4 + ty;
                    if x >= width || y >= height {
                        continue;
                    }
                    let i = (y as usize * width as usize + x as usize) * 4;
                    out[i..i + 4].copy_from_slice(&texels[(ty * 4 + tx) as usize]);
                }
            }
        }
    }
    out
}

fn get_bits(block: &[u8], start: usize, n: usize) -> u32 {
    let mut v = 0u32;
    for i in 0..n {
        let bit = start + i;
        let b = block[bit / 8];
        if (b >> (bit % 8)) & 1 != 0 {
            v |= 1 << i;
        }
    }
    v
}

fn interpolate_mode6(e0: [u8; 4], e1: [u8; 4], index: u32) -> [u8; 4] {
    const W: [i32; 16] = [0, 9, 18, 27, 37, 46, 55, 64, 74, 83, 92, 101, 111, 120, 129, 138];
    let w = W[index as usize];
    let mut out = [0u8; 4];
    for c in 0..4 {
        let v = (i32::from(e0[c]) * (64 - w) + i32::from(e1[c]) * w + 32) >> 6;
        out[c] = v.clamp(0, 255) as u8;
    }
    out
}

fn decode_bc7_block(block: &[u8], out: &mut [[u8; 4]; 16]) {
    // Mode 6 = 7-bit mode field value 1 (LSB-first).
    if get_bits(block, 0, 7) != 1 {
        for t in out.iter_mut() {
            *t = [0, 0, 0, 0];
        }
        return;
    }
    let mut pos = 7usize;
    let r0 = get_bits(block, pos, 7) as u8;
    pos += 7;
    let r1 = get_bits(block, pos, 7) as u8;
    pos += 7;
    let g0 = get_bits(block, pos, 7) as u8;
    pos += 7;
    let g1 = get_bits(block, pos, 7) as u8;
    pos += 7;
    let b0 = get_bits(block, pos, 7) as u8;
    pos += 7;
    let b1 = get_bits(block, pos, 7) as u8;
    pos += 7;
    let a0 = get_bits(block, pos, 7) as u8;
    pos += 7;
    let a1 = get_bits(block, pos, 7) as u8;
    pos += 7;
    let p0 = get_bits(block, pos, 1) as u8;
    pos += 1;
    let p1 = get_bits(block, pos, 1) as u8;
    pos += 1;
    let e0 = [
        (r0 << 1) | p0,
        (g0 << 1) | p0,
        (b0 << 1) | p0,
        (a0 << 1) | p0,
    ];
    let e1 = [
        (r1 << 1) | p1,
        (g1 << 1) | p1,
        (b1 << 1) | p1,
        (a1 << 1) | p1,
    ];
    let mut indices = [0u32; 16];
    indices[0] = get_bits(block, pos, 3);
    pos += 3;
    for i in 1..16 {
        indices[i] = get_bits(block, pos, 4);
        pos += 4;
    }
    for i in 0..16 {
        out[i] = interpolate_mode6(e0, e1, indices[i]);
    }
}

/// 颜色误差:每通道最大绝对差(8-bit)。
pub fn max_channel_delta(src: &[u8], dec: &[u8]) -> u8 {
    let n = src.len().min(dec.len());
    let mut m = 0u8;
    for i in 0..n {
        let d = src[i].abs_diff(dec[i]);
        if d > m {
            m = d;
        }
    }
    m
}

/// alpha coverage:源与解码在阈值 `thr` 上的覆盖率绝对差。
pub fn alpha_coverage_delta(src: &[u8], dec: &[u8], thr: u8) -> f64 {
    let pixels = src.len() / 4;
    if pixels == 0 {
        return 0.0;
    }
    let mut cs = 0usize;
    let mut cd = 0usize;
    for i in 0..pixels {
        if src[i * 4 + 3] >= thr {
            cs += 1;
        }
        if dec[i * 4 + 3] >= thr {
            cd += 1;
        }
    }
    let fs = cs as f64 / pixels as f64;
    let fd = cd as f64 / pixels as f64;
    (fs - fd).abs()
}

/// normal length:解码 RGB 作 XY(+假 Z)后 |n| 相对 1 的平均绝对偏差。
pub fn normal_length_mean_abs_dev(dec: &[u8]) -> f64 {
    let pixels = dec.len() / 4;
    if pixels == 0 {
        return 0.0;
    }
    let mut acc = 0.0f64;
    for i in 0..pixels {
        let x = (dec[i * 4] as f64 / 255.0) * 2.0 - 1.0;
        let y = (dec[i * 4 + 1] as f64 / 255.0) * 2.0 - 1.0;
        let z2 = (1.0 - x * x - y * y).max(0.0);
        let z = z2.sqrt();
        let len = (x * x + y * y + z * z).sqrt();
        acc += (len - 1.0).abs();
    }
    acc / pixels as f64
}
