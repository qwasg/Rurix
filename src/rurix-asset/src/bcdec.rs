//! 独立 BCn / ASTC 校验解码器(仅解码;供 M83 tolerance 对拍,避免 vendor 自证)。
//!
//! 覆盖真实 `basis_universal` UASTC→transcode 产出的 BC7 **mode 5 + mode 6**
//! (实测:常色块 → mode 6,渐变/normal 块 → mode 5)、BC4(单通道)、
//! BC5(双 BC4 = XY)。ASTC 4×4 为**结构校验**(块模式分类 + void-extent 判别),
//! 非全量像素解码 —— 见 VENDOR.md 诚实边界。
//!
//! 本文件不引用 `rurix-basis-sys`:独立实现是 tolerance 断言可信的前提。
//! //@ spec: RXS-0334

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

/// BC7 4-bit 权重表(== 上游 `g_bc7_weights4` 字面)。
const BC7_W4: [i32; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];
/// BC7 2-bit 权重表(== 上游 `g_bc7_weights2` 字面)。
const BC7_W2: [i32; 4] = [0, 21, 43, 64];

fn lerp_w(e0: [u8; 4], e1: [u8; 4], w: i32) -> [u8; 4] {
    let mut out = [0u8; 4];
    for c in 0..4 {
        let v = (i32::from(e0[c]) * (64 - w) + i32::from(e1[c]) * w + 32) >> 6;
        out[c] = v.clamp(0, 255) as u8;
    }
    out
}

/// BC7 mode 字段 = LSB-first 的一元前缀:mode m 表现为低 m 位为 0、第 m 位为 1。
fn bc7_mode(block: &[u8]) -> Option<u32> {
    (0..8u32).find(|&m| (block[0] >> m) & 1 == 1)
}

fn decode_bc7_block(block: &[u8], out: &mut [[u8; 4]; 16]) {
    match bc7_mode(block) {
        Some(6) => decode_bc7_mode6(block, out),
        Some(5) => decode_bc7_mode5(block, out),
        // 其它 mode 未覆盖:标记为透明黑,使 tolerance 断言必然超限(不静默充绿)。
        _ => {
            for t in out.iter_mut() {
                *t = [0, 0, 0, 0];
            }
        }
    }
}

/// BC7 mode 5:2 bit rotation + 7777 RGB endpoints + 8/8 alpha + 2-bit color
/// index + 2-bit alpha index(无 partition,单 subset)。
fn decode_bc7_mode5(block: &[u8], out: &mut [[u8; 4]; 16]) {
    let mut pos = 6usize; // mode 5 前缀占 6 bit(00000 1)
    let rotation = get_bits(block, pos, 2);
    pos += 2;
    let mut r = [0u8; 2];
    let mut g = [0u8; 2];
    let mut b = [0u8; 2];
    let mut a = [0u8; 2];
    for slot in r.iter_mut() {
        *slot = get_bits(block, pos, 7) as u8;
        pos += 7;
    }
    for slot in g.iter_mut() {
        *slot = get_bits(block, pos, 7) as u8;
        pos += 7;
    }
    for slot in b.iter_mut() {
        *slot = get_bits(block, pos, 7) as u8;
        pos += 7;
    }
    for slot in a.iter_mut() {
        *slot = get_bits(block, pos, 8) as u8;
        pos += 8;
    }
    // 7-bit 颜色分量左移补高位(无 p-bit)。
    let expand7 = |v: u8| (v << 1) | (v >> 6);
    let e0 = [expand7(r[0]), expand7(g[0]), expand7(b[0]), a[0]];
    let e1 = [expand7(r[1]), expand7(g[1]), expand7(b[1]), a[1]];

    // 颜色索引:31 bit(anchor 1 bit + 15×2 bit)。
    let mut cidx = [0u32; 16];
    cidx[0] = get_bits(block, pos, 1);
    pos += 1;
    for slot in cidx.iter_mut().skip(1) {
        *slot = get_bits(block, pos, 2);
        pos += 2;
    }
    // alpha 索引:同结构 31 bit。
    let mut aidx = [0u32; 16];
    aidx[0] = get_bits(block, pos, 1);
    pos += 1;
    for slot in aidx.iter_mut().skip(1) {
        *slot = get_bits(block, pos, 2);
        pos += 2;
    }

    for i in 0..16 {
        let c = lerp_w(e0, e1, BC7_W2[cidx[i] as usize]);
        let av = {
            let w = BC7_W2[aidx[i] as usize];
            let v = (i32::from(e0[3]) * (64 - w) + i32::from(e1[3]) * w + 32) >> 6;
            v.clamp(0, 255) as u8
        };
        let mut t = [c[0], c[1], c[2], av];
        // rotation:把 alpha 与指定颜色通道互换(BC7 规范)。
        match rotation {
            1 => t.swap(0, 3),
            2 => t.swap(1, 3),
            3 => t.swap(2, 3),
            _ => {}
        }
        out[i] = t;
    }
}

/// BC7 mode 6:7777.1.1 endpoints + 4-bit index(单 subset,含 alpha)。
fn decode_bc7_mode6(block: &[u8], out: &mut [[u8; 4]; 16]) {
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
    for slot in indices.iter_mut().skip(1) {
        *slot = get_bits(block, pos, 4);
        pos += 4;
    }
    for i in 0..16 {
        out[i] = lerp_w(e0, e1, BC7_W4[indices[i] as usize]);
    }
}

/// 解码单个 BC4 块(8 字节)为 16 个 8-bit 值(行主序 4×4)。
///
/// 布局:e0 u8 | e1 u8 | 16×3-bit 索引(48 bit,LSB-first)。
fn decode_bc4_block(block: &[u8], out: &mut [u8; 16]) {
    let e0 = block[0];
    let e1 = block[1];
    let mut bits = 0u64;
    for (i, &b) in block[2..8].iter().enumerate() {
        bits |= u64::from(b) << (i * 8);
    }
    for (i, slot) in out.iter_mut().enumerate() {
        let idx = ((bits >> (i * 3)) & 0x7) as u32;
        *slot = bc4_value(e0, e1, idx);
    }
}

/// BC4 索引 → 值(DX10 规范:e0>e1 为 6 插值档,否则 4 插值 + 0/255)。
fn bc4_value(e0: u8, e1: u8, idx: u32) -> u8 {
    let a = i32::from(e0);
    let b = i32::from(e1);
    if e0 > e1 {
        match idx {
            0 => e0,
            1 => e1,
            n => (((8 - (n as i32 - 1)) * a + (n as i32 - 1) * b) / 7).clamp(0, 255) as u8,
        }
    } else {
        match idx {
            0 => e0,
            1 => e1,
            6 => 0,
            7 => 255,
            n => (((6 - (n as i32 - 1)) * a + (n as i32 - 1) * b) / 5).clamp(0, 255) as u8,
        }
    }
}

/// BC4(单通道)→ 灰度 RGBA8(R=G=B=值,A=255)。
pub fn decode_bc4_r8(blocks: &[u8], width: u32, height: u32) -> Vec<u8> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];
    let mut bi = 0usize;
    for by in 0..bh {
        for bx in 0..bw {
            if bi + 8 > blocks.len() {
                return out;
            }
            let mut vals = [0u8; 16];
            decode_bc4_block(&blocks[bi..bi + 8], &mut vals);
            bi += 8;
            for ty in 0..4u32 {
                for tx in 0..4u32 {
                    let x = bx * 4 + tx;
                    let y = by * 4 + ty;
                    if x >= width || y >= height {
                        continue;
                    }
                    let v = vals[(ty * 4 + tx) as usize];
                    let i = (y as usize * width as usize + x as usize) * 4;
                    out[i..i + 4].copy_from_slice(&[v, v, v, 255]);
                }
            }
        }
    }
    out
}

/// BC5(两个 BC4 块 = XY)→ RGBA8(R=X、G=Y、B=0、A=255)。
pub fn decode_bc5_rg8(blocks: &[u8], width: u32, height: u32) -> Vec<u8> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];
    let mut bi = 0usize;
    for by in 0..bh {
        for bx in 0..bw {
            if bi + 16 > blocks.len() {
                return out;
            }
            let mut xs = [0u8; 16];
            let mut ys = [0u8; 16];
            decode_bc4_block(&blocks[bi..bi + 8], &mut xs);
            decode_bc4_block(&blocks[bi + 8..bi + 16], &mut ys);
            bi += 16;
            for ty in 0..4u32 {
                for tx in 0..4u32 {
                    let x = bx * 4 + tx;
                    let y = by * 4 + ty;
                    if x >= width || y >= height {
                        continue;
                    }
                    let t = (ty * 4 + tx) as usize;
                    let i = (y as usize * width as usize + x as usize) * 4;
                    out[i..i + 4].copy_from_slice(&[xs[t], ys[t], 0, 255]);
                }
            }
        }
    }
    out
}

/// ASTC 4×4 块分类(结构校验腿;非全量像素解码)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstcBlockKind {
    /// void-extent(常色块):低 9 位 == 0x1FC。
    VoidExtent,
    /// 普通带权重块(真实 weight/endpoint 载荷)。
    Weighted,
    /// 保留/非法块模式(低 9 位全 1 等)。
    Reserved,
}

/// 分类 ASTC 4×4 块序列。用于断言"非全 void-extent 充绿"。
pub fn classify_astc4x4(blocks: &[u8]) -> Vec<AstcBlockKind> {
    let mut v = Vec::with_capacity(blocks.len() / 16);
    for blk in blocks.chunks_exact(16) {
        let m = u16::from_le_bytes([blk[0], blk[1]]);
        let low9 = m & 0x1FF;
        // void-extent:bits[8:0] == 111111100
        if low9 == 0x1FC {
            v.push(AstcBlockKind::VoidExtent);
        } else if (m & 0x0003) == 0x0000 && (m & 0x01FC) == 0x01FC {
            v.push(AstcBlockKind::Reserved);
        } else {
            v.push(AstcBlockKind::Weighted);
        }
    }
    v
}

/// ASTC 4×4 LDR 块结构统计(仅分类,不做全量像素解码)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AstcBlockStats {
    pub total: usize,
    /// void-extent 块(常色块):低 9 位 == 0x1FC。
    pub void_extent: usize,
    /// 带权重网格的真实压缩块(非 void-extent)。
    pub weighted: usize,
    /// 逐块字节全零(占位嫌疑)。
    pub all_zero: usize,
}

/// 分类 ASTC 4×4 块流。判据用途:`weighted > 0` 证明编码器产出**真实权重块**,
/// 而非整幅 void-extent / 均值敷衍。
pub fn astc4x4_block_stats(blocks: &[u8]) -> AstcBlockStats {
    let mut s = AstcBlockStats::default();
    for blk in blocks.chunks_exact(16) {
        s.total += 1;
        if blk.iter().all(|&b| b == 0) {
            s.all_zero += 1;
        }
        let mode = u16::from_le_bytes([blk[0], blk[1]]);
        if (mode & 0x1FF) == 0x1FC {
            s.void_extent += 1;
        } else {
            s.weighted += 1;
        }
    }
    s
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

/// 颜色误差:仅在给定通道下标(RGBA 序)上取每通道最大绝对差。
///
/// 用于 BC5(仅 R/G 有效)与 BC4(仅 R 有效)腿:其余通道由格式定义为
/// 重建值(BC5 的 Z / BC4 的 GBA),与源逐字节比对无语义。
pub fn max_channel_delta_channels(src: &[u8], dec: &[u8], channels: &[usize]) -> u8 {
    let pixels = (src.len() / 4).min(dec.len() / 4);
    let mut m = 0u8;
    for p in 0..pixels {
        for &c in channels {
            debug_assert!(c < 4);
            let d = src[p * 4 + c].abs_diff(dec[p * 4 + c]);
            if d > m {
                m = d;
            }
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
