//! 独立 BCn / ASTC 校验解码器(仅解码;供 M83 tolerance 对拍,避免 vendor 自证)。
//!
//! 覆盖真实 `basis_universal` UASTC→transcode 产出空间的 BC7 **全 8 mode**
//! (UASTC 模式与 BC7 mode 0-7 一一映射;bistro 实纹理对拍实测命中 0/1/2/3/5/6/7——
//! mode 0-3/7 覆盖为 G31+ 波 C Task C14 追加,布局镜像 vendor
//! `unpack_bc7_mode0_2` / `unpack_bc7_mode1_3_7` / `unpack_bc7_mode4_5` 语义)、
//! BC4(单通道)、BC5(双 BC4 = XY)。ASTC 4×4 为**结构校验**(块模式分类 +
//! void-extent 判别),非全量像素解码 —— 见 VENDOR.md 诚实边界。
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
/// BC7 3-bit 权重表(== 上游 `g_bc7_weights3` 字面)。
const BC7_W3: [i32; 8] = [0, 9, 18, 27, 37, 46, 55, 64];
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
        Some(4) => decode_bc7_mode4(block, out),
        Some(7) => decode_bc7_mode7(block, out),
        Some(m @ (0..=3)) => decode_bc7_mode0_3(block, out, m),
        // 不可达(一元前缀必中 0..=7):保底透明黑(不静默充绿)。
        _ => {
            for t in out.iter_mut() {
                *t = [0, 0, 0, 0];
            }
        }
    }
}

/// BC7 端点 dequant(镜像 vendor `bc7_dequant` 双形):
/// 带 p-bit — val=(v<<1)|pbit 后按 total_bits=bits+1 复制扩展到 8-bit;
/// 无 p-bit — 按 bits 复制扩展到 8-bit。
fn bc7_dequant(v: u8, pbit: Option<u8>, bits: usize) -> u8 {
    let (mut val, total) = match pbit {
        Some(p) => (((v << 1) | p) as u32, bits + 1),
        None => (v as u32, bits),
    };
    val <<= 8 - total;
    val |= val >> total;
    val.min(255) as u8
}

/// BC7 mode 0/1/2/3(多子集面;布局镜像 vendor
/// `unpack_bc7_mode0_2` / `unpack_bc7_mode1_3_7`):
/// - mode 0: 3 子集,part 4b,RGB 444 + 逐端点 p-bit(6),3-bit 权重(3 anchor);
/// - mode 1: 2 子集,part 6b,RGB 666 + 共享 p-bit(2,按端点对),3-bit 权重(2 anchor);
/// - mode 2: 3 子集,part 6b,RGB 555 无 p-bit,2-bit 权重(3 anchor);
/// - mode 3: 2 子集,part 6b,RGB 777 + 逐端点 p-bit(4),2-bit 权重(2 anchor)。
/// alpha 恒 255(RGB-only mode)。
fn decode_bc7_mode0_3(block: &[u8], out: &mut [[u8; 4]; 16], mode: u32) {
    let (subsets, part_bits, weight_bits, endpoint_bits, pbit_per_endpoint, pbit_shared) =
        match mode {
            0 => (3usize, 4usize, 3usize, 4usize, true, false),
            1 => (2, 6, 3, 6, false, true),
            2 => (3, 6, 2, 5, false, false),
            _ => (2, 6, 2, 7, true, false),
        };
    let mut pos = (mode + 1) as usize;
    let part = get_bits(block, pos, part_bits) as usize;
    pos += part_bits;
    let n_ep = subsets * 2;
    let mut ep = [[0u8; 3]; 6];
    for c in 0..3 {
        for e in ep.iter_mut().take(n_ep) {
            e[c] = get_bits(block, pos, endpoint_bits) as u8;
            pos += endpoint_bits;
        }
    }
    let n_pbits = if pbit_per_endpoint {
        n_ep
    } else if pbit_shared {
        subsets
    } else {
        0
    };
    let mut pbits = [0u8; 6];
    for p in pbits.iter_mut().take(n_pbits) {
        *p = get_bits(block, pos, 1) as u8;
        pos += 1;
    }
    // anchor 集合:子集 0 anchor = 0;子集 1 anchor 按子集数取表;
    // 子集 2 anchor 仅 3 子集 mode 取 third_2。
    let a1 = if subsets == 3 {
        BC7_ANCHOR_THIRD_1[part] as usize
    } else {
        BC7_ANCHOR_SECOND_SUBSET[part] as usize
    };
    let a2 = if subsets == 3 {
        BC7_ANCHOR_THIRD_2[part] as usize
    } else {
        usize::MAX
    };
    let wtab: &[i32] = if weight_bits == 3 { &BC7_W3 } else { &BC7_W2 };
    let mut widx = [0u32; 16];
    for (i, slot) in widx.iter_mut().enumerate() {
        let n = if i == 0 || i == a1 || i == a2 {
            weight_bits - 1
        } else {
            weight_bits
        };
        *slot = get_bits(block, pos, n);
        pos += n;
    }
    debug_assert_eq!(pos, 128);
    // 端点 dequant(RGB;alpha 恒 255)。
    let mut e = [[0u8; 4]; 6];
    for (i, item) in e.iter_mut().enumerate().take(n_ep) {
        let pb = if pbit_per_endpoint {
            Some(pbits[i])
        } else if pbit_shared {
            Some(pbits[i >> 1])
        } else {
            None
        };
        for c in 0..3 {
            item[c] = bc7_dequant(ep[i][c], pb, endpoint_bits);
        }
        item[3] = 255;
    }
    // 逐子集调色板。
    let mut pal = [[[0u8; 4]; 8]; 3];
    for (s, ps) in pal.iter_mut().enumerate().take(subsets) {
        for (i, item) in ps.iter_mut().enumerate().take(wtab.len()) {
            *item = lerp_w(e[s * 2], e[s * 2 + 1], wtab[i]);
        }
    }
    let ptab: &[u8] = if subsets == 3 {
        &BC7_PARTITION3
    } else {
        &BC7_PARTITION2
    };
    for i in 0..16 {
        let s = ptab[part * 16 + i] as usize;
        out[i] = pal[s][widx[i] as usize];
    }
}

/// BC7 mode 4:1 子集,2-bit rotation + 1-bit index_mode + RGB 555/A 66
/// (无 p-bit);color/alpha 权重精度按 index_mode 互换(2↔3 bit,首读集随
/// index_mode 切换;镜像 vendor `unpack_bc7_mode4_5` mode=4 分支)。
fn decode_bc7_mode4(block: &[u8], out: &mut [[u8; 4]; 16]) {
    let mut pos = 5usize; // mode 4 前缀占 5 bit
    let rotation = get_bits(block, pos, 2);
    pos += 2;
    let index_mode = get_bits(block, pos, 1) as usize;
    pos += 1;
    // 端点:[c][e] 序,RGB 5 bit / A 6 bit。
    let mut ep = [[0u8; 4]; 2];
    for c in 0..4 {
        for e in ep.iter_mut() {
            let n = if c == 3 { 6 } else { 5 };
            e[c] = get_bits(block, pos, n) as u8;
            pos += n;
        }
    }
    // color 权重精度 = index_mode ? 3 : 2;alpha = index_mode ? 2 : 3。
    let cbits = if index_mode == 1 { 3 } else { 2 };
    let abits = if index_mode == 1 { 2 } else { 3 };
    let mut cidx = [0u32; 16];
    let mut aidx = [0u32; 16];
    // 首读集:index_mode=0 → color;index_mode=1 → alpha(各 anchor 仅 i==0 减 1 bit)。
    for i in 0..16 {
        let n = (if index_mode == 1 { abits } else { cbits }) - usize::from(i == 0);
        let v = get_bits(block, pos, n);
        pos += n;
        if index_mode == 1 {
            aidx[i] = v;
        } else {
            cidx[i] = v;
        }
    }
    for i in 0..16 {
        let n = (if index_mode == 1 { cbits } else { abits }) - usize::from(i == 0);
        let v = get_bits(block, pos, n);
        pos += n;
        if index_mode == 1 {
            cidx[i] = v;
        } else {
            aidx[i] = v;
        }
    }
    debug_assert_eq!(pos, 128);
    let mut e = [[0u8; 4]; 2];
    for i in 0..2 {
        for c in 0..4 {
            e[i][c] = bc7_dequant(ep[i][c], None, if c == 3 { 6 } else { 5 });
        }
    }
    let ctab: &[i32] = if cbits == 3 { &BC7_W3 } else { &BC7_W2 };
    let atab: &[i32] = if abits == 3 { &BC7_W3 } else { &BC7_W2 };
    let mut cpal = [[0u8; 4]; 8];
    for (i, item) in cpal.iter_mut().enumerate().take(ctab.len()) {
        *item = lerp_w(e[0], e[1], ctab[i]);
    }
    let mut apal = [0u8; 8];
    for (i, item) in apal.iter_mut().enumerate().take(atab.len()) {
        let w = atab[i];
        let v = (i32::from(e[0][3]) * (64 - w) + i32::from(e[1][3]) * w + 32) >> 6;
        *item = v.clamp(0, 255) as u8;
    }
    for i in 0..16 {
        let c = cpal[cidx[i] as usize];
        let mut t = [c[0], c[1], c[2], apal[aidx[i] as usize]];
        match rotation {
            1 => t.swap(0, 3),
            2 => t.swap(1, 3),
            3 => t.swap(2, 3),
            _ => {}
        }
        out[i] = t;
    }
}

/// BC7 二子集划分表(64×16;镜像 vendor `g_bc7_partition2` 字面,
/// basis_universal 1.16.4 transcoder/basisu_transcoder.cpp L11453-11463)。
#[rustfmt::skip]
const BC7_PARTITION2: [u8; 64 * 16] = [
    0,0,1,1,0,0,1,1,0,0,1,1,0,0,1,1, 0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,1, 0,1,1,1,0,1,1,1,0,1,1,1,0,1,1,1, 0,0,0,1,0,0,1,1,0,0,1,1,0,1,1,1, 0,0,0,0,0,0,0,1,0,0,0,1,0,0,1,1, 0,0,1,1,0,1,1,1,0,1,1,1,1,1,1,1, 0,0,0,1,0,0,1,1,0,1,1,1,1,1,1,1, 0,0,0,0,0,0,0,1,0,0,1,1,0,1,1,1,
    0,0,0,0,0,0,0,0,0,0,0,1,0,0,1,1, 0,0,1,1,0,1,1,1,1,1,1,1,1,1,1,1, 0,0,0,0,0,0,0,1,0,1,1,1,1,1,1,1, 0,0,0,0,0,0,0,0,0,0,0,1,0,1,1,1, 0,0,0,1,0,1,1,1,1,1,1,1,1,1,1,1, 0,0,0,0,0,0,0,0,1,1,1,1,1,1,1,1, 0,0,0,0,1,1,1,1,1,1,1,1,1,1,1,1, 0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,
    0,0,0,0,1,0,0,0,1,1,1,0,1,1,1,1, 0,1,1,1,0,0,0,1,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,1,0,0,0,1,1,1,0, 0,1,1,1,0,0,1,1,0,0,0,1,0,0,0,0, 0,0,1,1,0,0,0,1,0,0,0,0,0,0,0,0, 0,0,0,0,1,0,0,0,1,1,0,0,1,1,1,0, 0,0,0,0,0,0,0,0,1,0,0,0,1,1,0,0, 0,1,1,1,0,0,1,1,0,0,1,1,0,0,0,1,
    0,0,1,1,0,0,0,1,0,0,0,1,0,0,0,0, 0,0,0,0,1,0,0,0,1,0,0,0,1,1,0,0, 0,1,1,0,0,1,1,0,0,1,1,0,0,1,1,0, 0,0,1,1,0,1,1,0,0,1,1,0,1,1,0,0, 0,0,0,1,0,1,1,1,1,1,1,0,1,0,0,0, 0,0,0,0,1,1,1,1,1,1,1,1,0,0,0,0, 0,1,1,1,0,0,0,1,1,0,0,0,1,1,1,0, 0,0,1,1,1,0,0,1,1,0,0,1,1,1,0,0,
    0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1, 0,0,0,0,1,1,1,1,0,0,0,0,1,1,1,1, 0,1,0,1,1,0,1,0,0,1,0,1,1,0,1,0, 0,0,1,1,0,0,1,1,1,1,0,0,1,1,0,0, 0,0,1,1,1,1,0,0,0,0,1,1,1,1,0,0, 0,1,0,1,0,1,0,1,1,0,1,0,1,0,1,0, 0,1,1,0,1,0,0,1,0,1,1,0,1,0,0,1, 0,1,0,1,1,0,1,0,1,0,1,0,0,1,0,1,
    0,1,1,1,0,0,1,1,1,1,0,0,1,1,1,0, 0,0,0,1,0,0,1,1,1,1,0,0,1,0,0,0, 0,0,1,1,0,0,1,0,0,1,0,0,1,1,0,0, 0,0,1,1,1,0,1,1,1,1,0,1,1,1,0,0, 0,0,1,1,0,1,0,0,1,1,0,0,1,0,1,0, 0,1,1,1,1,0,0,1,1,0,0,0,0,1,1,0, 0,1,1,0,0,1,1,0,1,0,0,1,1,0,0,1, 0,0,0,0,0,1,1,0,0,1,1,0,0,0,0,0,
    0,1,0,0,1,1,1,0,0,1,0,0,0,0,0,0, 0,0,1,0,0,1,1,1,0,0,1,0,0,0,0,0, 0,0,0,0,0,0,1,0,0,1,1,1,0,0,1,0, 0,0,0,0,0,1,0,0,1,1,1,0,0,1,0,0, 0,1,1,0,1,1,0,0,1,0,0,1,0,0,1,1, 0,0,1,1,0,1,1,0,1,1,0,0,1,0,0,1, 0,1,1,0,0,0,1,1,1,0,0,1,1,1,0,0, 0,0,1,1,1,0,0,1,1,1,0,0,0,1,1,0,
    0,1,1,0,1,1,0,0,1,1,0,0,1,0,0,1, 0,1,1,0,0,0,1,1,0,0,1,1,1,0,0,1, 0,1,1,1,1,1,1,0,1,0,0,0,0,0,0,1, 0,0,0,1,1,0,0,0,1,1,1,0,0,1,1,1, 0,0,0,0,1,1,1,1,0,0,1,1,0,0,1,1, 0,0,1,1,0,0,1,1,1,1,1,1,0,0,0,0, 0,0,1,0,0,0,1,0,1,1,1,0,1,1,1,0, 0,1,0,0,0,1,0,0,0,1,1,1,0,1,1,1,
];

/// BC7 二子集第二 anchor 索引表(64;镜像 vendor
/// `g_bc7_table_anchor_index_second_subset` 字面,L11477)。
#[rustfmt::skip]
const BC7_ANCHOR_SECOND_SUBSET: [u8; 64] = [
    15,15,15,15,15,15,15,15, 15,15,15,15,15,15,15,15,
    15, 2, 8, 2, 2, 8, 8,15,  2, 8, 2, 2, 8, 8, 2, 2,
    15,15, 6, 8, 2, 8,15,15,  2, 8, 2, 2, 2,15,15, 6,
     6, 2, 6, 8,15,15, 2, 2, 15,15,15,15,15, 2, 2,15,
];

/// BC7 三子集划分表(64×16;镜像 vendor `g_bc7_partition3` 字面,L11465-11475)。
#[rustfmt::skip]
const BC7_PARTITION3: [u8; 64 * 16] = [
    0,0,1,1,0,0,1,1,0,2,2,1,2,2,2,2, 0,0,0,1,0,0,1,1,2,2,1,1,2,2,2,1, 0,0,0,0,2,0,0,1,2,2,1,1,2,2,1,1, 0,2,2,2,0,0,2,2,0,0,1,1,0,1,1,1, 0,0,0,0,0,0,0,0,1,1,2,2,1,1,2,2, 0,0,1,1,0,0,1,1,0,0,2,2,0,0,2,2, 0,0,2,2,0,0,2,2,1,1,1,1,1,1,1,1, 0,0,1,1,2,2,1,1,2,2,1,1,2,2,1,1,
    0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2, 0,0,0,0,1,1,1,1,1,1,1,1,2,2,2,2, 0,0,1,2,0,0,1,2,0,0,1,2,0,0,1,2, 0,1,1,2,0,1,1,2,0,1,1,2,0,1,1,2, 0,1,2,2,0,1,2,2,0,1,2,2,0,1,2,2, 0,1,1,1,0,1,1,2,1,2,2,1,2,2,2,0, 0,0,1,1,2,2,0,0,2,2,0,0,2,2,2,0, 0,1,2,2,1,1,2,2,1,2,2,2,1,2,2,2,
    0,0,0,1,0,0,1,1,0,1,1,2,1,1,2,2, 0,1,1,1,0,0,1,2,0,0,1,2,2,0,0,0, 0,0,0,0,0,1,1,2,2,1,1,2,2,1,1,2, 0,0,2,2,0,0,2,2,0,0,2,2,1,1,1,1, 0,1,1,1,0,1,1,1,0,2,2,2,0,2,2,2, 0,0,0,1,0,0,0,1,2,2,2,1,2,2,2,1, 0,1,1,0,1,2,2,1,1,2,2,1,2,2,1,1, 0,1,1,0,0,1,1,0,0,2,2,1,0,2,2,1,
    0,1,2,2,0,1,2,2,0,0,1,1,0,0,0,0, 0,0,1,2,0,1,2,1,1,2,2,2,2,2,2,2, 0,1,1,0,1,2,2,1,1,2,2,1,0,1,1,0, 0,0,0,1,1,2,2,1,1,2,2,1,1,2,2,1, 0,0,2,2,1,1,0,2,1,1,0,2,0,0,2,2, 0,1,1,0,0,1,1,0,2,0,0,2,2,2,2,2, 0,0,0,0,2,0,0,0,2,2,1,1,2,2,2,1, 0,1,1,0,0,2,2,1,1,2,2,0,0,1,1,1,
    0,0,0,0,0,0,0,2,1,1,2,2,1,2,2,2, 0,2,2,2,0,0,2,2,0,0,1,2,0,0,1,1, 0,0,1,1,0,0,1,2,0,0,2,2,0,2,2,2, 0,1,2,0,1,2,0,1,2,0,0,1,2,0,1,2, 0,1,1,1,1,1,1,1,0,1,1,1,0,0,0,0, 0,1,2,0,1,2,0,1,2,0,1,2,0,1,2,0, 0,1,2,0,2,0,1,2,1,2,0,1,0,1,2,0, 0,1,1,2,2,0,0,1,2,2,0,0,1,2,2,1,
    0,0,1,1,1,1,2,2,2,2,0,0,0,0,1,1, 0,1,0,1,0,1,0,1,2,2,2,2,2,2,2,2, 0,0,0,0,0,0,0,0,2,1,2,1,2,1,2,1, 0,0,2,2,1,1,2,2,0,0,2,2,1,1,2,2, 0,0,2,2,0,0,1,1,0,0,2,2,0,0,1,1, 0,2,2,2,0,1,2,2,1,0,2,2,0,1,2,2, 0,1,0,1,2,2,2,2,2,2,2,2,0,1,0,1, 0,0,0,2,1,2,1,2,1,2,1,2,1,2,1,2,
    0,1,0,1,0,1,0,1,0,1,0,1,2,2,2,2, 0,2,2,2,0,1,1,1,0,2,2,2,0,1,1,1, 0,0,0,2,1,1,1,2,0,0,0,2,1,1,1,2, 0,0,0,0,2,1,1,2,2,1,1,2,1,1,1,2, 0,2,2,2,0,1,1,1,0,1,1,1,0,2,2,2, 0,1,1,0,0,2,2,2,1,2,0,0,1,1,0,2, 0,1,1,0,1,1,0,0,1,1,0,2,2,2,2,2, 0,0,0,0,0,0,0,0,2,1,1,2,2,1,1,2,
    0,1,1,0,0,1,1,0,2,2,2,2,2,2,2,2, 0,0,2,2,0,0,1,1,0,0,2,2,0,0,2,2, 0,1,1,2,2,1,1,2,2,1,1,0,0,2,2,2, 0,0,0,0,0,0,0,0,0,0,0,0,2,1,1,2, 0,0,0,2,0,0,0,1,0,0,0,2,0,0,0,1, 0,2,2,2,1,2,2,2,0,2,2,2,1,2,2,2, 0,1,0,1,2,2,2,2,2,2,2,2,2,2,2,2, 0,1,1,1,2,0,1,1,2,2,0,1,2,2,2,0,
];

/// BC7 三子集 anchor 表(64×2;镜像 vendor
/// `g_bc7_table_anchor_index_third_subset_{1,2}` 字面,L11479-11487)。
#[rustfmt::skip]
const BC7_ANCHOR_THIRD_1: [u8; 64] = [
     3, 3,15,15, 8, 3,15,15,  8, 8, 6, 6, 6, 5, 3, 3,
     3, 3, 8,15, 3, 3, 6,10,  5, 8, 8, 6, 8, 5,15,15,
     8,15, 3, 5, 6,10, 8,15, 15, 3,15, 5,15,15,15,15,
     3,15, 5, 5, 5, 8, 5,10,  5,10, 8,13,15,12, 3, 3,
];
#[rustfmt::skip]
const BC7_ANCHOR_THIRD_2: [u8; 64] = [
    15, 8, 8, 3,15,15, 3, 8, 15,15,15,15,15,15,15, 8,
    15, 8,15, 3,15, 8,15, 8,  3,15, 6,10,15,15,10, 8,
    15, 3,15,10,10, 8, 9,10,  6,15, 8,15, 3, 6, 6, 8,
    15, 3,15,15,15,15,15,15, 15,15,15,15, 3,15,15, 8,
];

/// BC7 mode 7:二子集 6-bit partition + RGBA 5555 端点(逐端点 p-bit)+
/// 2-bit 权重(双 anchor)。布局镜像 vendor `unpack_bc7_mode1_3_7`(mode=7 分支)。
fn decode_bc7_mode7(block: &[u8], out: &mut [[u8; 4]; 16]) {
    let mut pos = 8usize; // mode 7 前缀占 8 bit(0000000 1)
    let part = get_bits(block, pos, 6) as usize;
    pos += 6;
    // 端点:[c][e] 序(R0..R3,G0..G3,B0..B3,A0..A3),各 5 bit。
    let mut ep = [[0u8; 4]; 4];
    for c in 0..4 {
        for e in 0..4 {
            ep[e][c] = get_bits(block, pos, 5) as u8;
            pos += 5;
        }
    }
    let mut pbits = [0u8; 4];
    for p in pbits.iter_mut() {
        *p = get_bits(block, pos, 1) as u8;
        pos += 1;
    }
    // 权重:16×2 bit,anchor(i==0 与 i==anchor2[part])为 1 bit。
    let anchor2 = BC7_ANCHOR_SECOND_SUBSET[part] as usize;
    let mut widx = [0u32; 16];
    for (i, slot) in widx.iter_mut().enumerate() {
        let n = if i == 0 || i == anchor2 { 1 } else { 2 };
        *slot = get_bits(block, pos, n);
        pos += n;
    }
    debug_assert_eq!(pos, 128);
    // 端点 dequant(5+1 → 8-bit)。
    let mut e = [[0u8; 4]; 4];
    for i in 0..4 {
        for c in 0..4 {
            e[i][c] = bc7_dequant(ep[i][c], Some(pbits[i]), 5);
        }
    }
    // 逐子集调色板(2-bit 权重插值)。
    let mut pal = [[[0u8; 4]; 4]; 2];
    for s in 0..2 {
        for (i, item) in pal[s].iter_mut().enumerate() {
            *item = lerp_w(e[s * 2], e[s * 2 + 1], BC7_W2[i]);
        }
    }
    for i in 0..16 {
        let s = BC7_PARTITION2[part * 16 + i] as usize;
        out[i] = pal[s][widx[i] as usize];
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

// ─────────────────── BC1(DXT1)/ BC3(DXT5) 与 DDS 容器(G11.3 U2/R1 修复面) ───────────────────
//
// Bistro 语料实测枚举(144 张 `.dds`,G11.3 资产面实盘点):legacy FourCC
// `DXT1`(BC1)×54 / `DXT5`(BC3)×20 / `ATI2`(BC5)×70;零 DX10 扩展头。
// 本段补齐 BC1/BC3 解码与 DDS 容器解析(legacy FourCC + DX10 头双形),
// 支撑 baseColor/normal 纹理在 host 参考管线的真实采样(G10-N7 承接锚兑现)。

/// RGB565 通道扩展为 8-bit(位复制法:5→8 = v<<3|v>>2;6→8 = v<<2|v>>4)。
fn rgb565_to_rgba8(c: u16) -> [u8; 4] {
    let r = ((c >> 11) & 0x1f) as u8;
    let g = ((c >> 5) & 0x3f) as u8;
    let b = (c & 0x1f) as u8;
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
        255,
    ]
}

/// 解码单个 BC1 块(8 字节)为 16 个 RGBA8 texel(行主序 4×4)。
///
/// 布局:`c0 u16 LE | c1 u16 LE`(RGB565)+ 16×2-bit 索引(u32 LE,LSB-first)。
/// `c0 > c1`(无符号)= 四色模式(两级 1/3 插值,整数截断除法);
/// `c0 <= c1` = 三色 + 透明黑模式(索引 3 = RGBA 全 0)。
fn decode_bc1_block(block: &[u8], out: &mut [[u8; 4]; 16]) {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let idx_bits = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    let p0 = rgb565_to_rgba8(c0);
    let p1 = rgb565_to_rgba8(c1);
    let mut lut = [[0u8; 4]; 4];
    lut[0] = p0;
    lut[1] = p1;
    if c0 > c1 {
        for ch in 0..3 {
            lut[2][ch] = ((2 * u32::from(p0[ch]) + u32::from(p1[ch])) / 3) as u8;
            lut[3][ch] = ((u32::from(p0[ch]) + 2 * u32::from(p1[ch])) / 3) as u8;
        }
        lut[2][3] = 255;
        lut[3][3] = 255;
    } else {
        for ch in 0..3 {
            lut[2][ch] = ((u32::from(p0[ch]) + u32::from(p1[ch])) / 2) as u8;
        }
        lut[2][3] = 255;
        lut[3] = [0, 0, 0, 0];
    }
    for (i, texel) in out.iter_mut().enumerate() {
        *texel = lut[((idx_bits >> (i * 2)) & 0x3) as usize];
    }
}

/// 解码 BC1(DXT1)块字节为 RGBA8(行主序)。`blocks` 长度须 = ceil(w/4)*ceil(h/4)*8。
pub fn decode_bc1_rgba8(blocks: &[u8], width: u32, height: u32) -> Vec<u8> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];
    let mut bi = 0usize;
    for by in 0..bh {
        for bx in 0..bw {
            if bi + 8 > blocks.len() {
                return out;
            }
            let mut texels = [[0u8; 4]; 16];
            decode_bc1_block(&blocks[bi..bi + 8], &mut texels);
            bi += 8;
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

/// 解码 BC3(DXT5)块字节为 RGBA8(行主序)。
/// 块 = BC4 形 alpha 块(8 字节)+ BC1 形颜色块(8 字节,颜色恒四色模式)。
/// `blocks` 长度须 = ceil(w/4)*ceil(h/4)*16。
pub fn decode_bc3_rgba8(blocks: &[u8], width: u32, height: u32) -> Vec<u8> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];
    let mut bi = 0usize;
    for by in 0..bh {
        for bx in 0..bw {
            if bi + 16 > blocks.len() {
                return out;
            }
            let mut alpha = [0u8; 16];
            decode_bc4_block(&blocks[bi..bi + 8], &mut alpha);
            // 颜色块按四色模式解码:c0/c1 大端序强制在四色档——构造
            // c0>c1 的等价格局不如直接展开;此处复用 BC1 块解码再以
            // BC3 规范覆写(BC3 颜色块 c0<=c1 时索引 3 仍为第四插值色,
            // 不透明)——故独立展开而非调 decode_bc1_block。
            let cb = &blocks[bi + 8..bi + 16];
            let c0 = u16::from_le_bytes([cb[0], cb[1]]);
            let c1 = u16::from_le_bytes([cb[2], cb[3]]);
            let idx_bits = u32::from_le_bytes([cb[4], cb[5], cb[6], cb[7]]);
            let p0 = rgb565_to_rgba8(c0);
            let p1 = rgb565_to_rgba8(c1);
            let mut lut = [[0u8; 4]; 4];
            lut[0] = p0;
            lut[1] = p1;
            for ch in 0..3 {
                lut[2][ch] = ((2 * u32::from(p0[ch]) + u32::from(p1[ch])) / 3) as u8;
                lut[3][ch] = ((u32::from(p0[ch]) + 2 * u32::from(p1[ch])) / 3) as u8;
            }
            for ty in 0..4u32 {
                for tx in 0..4u32 {
                    let x = bx * 4 + tx;
                    let y = by * 4 + ty;
                    if x >= width || y >= height {
                        continue;
                    }
                    let t = (ty * 4 + tx) as usize;
                    let mut px = lut[((idx_bits >> (t * 2)) & 0x3) as usize];
                    px[3] = alpha[t];
                    let i = (y as usize * width as usize + x as usize) * 4;
                    out[i..i + 4].copy_from_slice(&px);
                }
            }
            bi += 16;
        }
    }
    out
}

/// DDS 像素格式(G11.3 消费闭集;BC2/DXT3 等未覆盖格式 fail-closed 显式拒绝)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdsFormat {
    Bc1,
    Bc3,
    Bc4,
    Bc5,
    Bc7,
}

impl DdsFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            DdsFormat::Bc1 => "bc1",
            DdsFormat::Bc3 => "bc3",
            DdsFormat::Bc4 => "bc4",
            DdsFormat::Bc5 => "bc5",
            DdsFormat::Bc7 => "bc7",
        }
    }

    /// 每 4×4 块字节数(BC1/BC4=8,BC3/BC5/BC7=16;G31+ 波 C Task C14 公开面)。
    pub fn block_bytes(self) -> usize {
        match self {
            DdsFormat::Bc1 | DdsFormat::Bc4 => 8,
            DdsFormat::Bc3 | DdsFormat::Bc5 | DdsFormat::Bc7 => 16,
        }
    }
}

/// DDS 解码产物(mip 0;RGBA8 行主序)。
#[derive(Debug, Clone)]
pub struct DdsImage {
    pub width: u32,
    pub height: u32,
    pub format: DdsFormat,
    pub mip_count: u32,
    pub rgba8: Vec<u8>,
}

fn dds_u32(bytes: &[u8], off: usize) -> Result<u32, String> {
    let b = bytes
        .get(off..off + 4)
        .ok_or_else(|| format!("DDS 头截断 @0x{off:x}"))?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// 解码 DDS 容器(mip 0)为 RGBA8。
///
/// 容器两形:legacy FourCC(`DXT1`/`DXT5`/`ATI1`/`BC4U`/`ATI2`/`BC5U`/`BC5S`)
/// 与 DX10 扩展头(DXGI BC1/BC3/BC4/BC5/BC7 子集,UNORM 与 SRGB 同解码——
/// 色域语义归消费方)。非闭集格式 / 头截断 / 体长不符一律 fail-closed。
pub fn decode_dds(bytes: &[u8]) -> Result<DdsImage, String> {
    if bytes.len() < 128 {
        return Err(format!("DDS 头长不足 128: {}", bytes.len()));
    }
    if &bytes[0..4] != b"DDS " {
        return Err("非 DDS magic".to_owned());
    }
    if dds_u32(bytes, 4)? != 124 {
        return Err("DDS header.size ≠ 124".to_owned());
    }
    let height = dds_u32(bytes, 12)?;
    let width = dds_u32(bytes, 16)?;
    let mip_count = dds_u32(bytes, 28).unwrap_or(1).max(1);
    if width == 0 || height == 0 {
        return Err("DDS 尺寸为零".to_owned());
    }
    if dds_u32(bytes, 76)? != 32 {
        return Err("DDS ddspf.size ≠ 32".to_owned());
    }
    let fourcc = &bytes[84..88];
    let mut data_off = 128usize;
    let format = match fourcc {
        b"DXT1" => DdsFormat::Bc1,
        b"DXT5" => DdsFormat::Bc3,
        b"ATI1" | b"BC4U" => DdsFormat::Bc4,
        b"ATI2" | b"BC5U" | b"BC5S" => DdsFormat::Bc5,
        b"DX10" => {
            if bytes.len() < 148 {
                return Err("DDS DX10 扩展头截断".to_owned());
            }
            data_off = 148;
            // DXGI_FORMAT 子集:71/72=BC1, 77/78=BC3, 80/81/82=BC4, 83/84=BC5,
            // 98/99=BC7(UNORM/SRGB/TYPELESS 同块解码)。
            match dds_u32(bytes, 128)? {
                71 | 72 | 73 => DdsFormat::Bc1,
                77 | 78 | 79 => DdsFormat::Bc3,
                80 | 81 | 82 => DdsFormat::Bc4,
                83 | 84 | 85 => DdsFormat::Bc5,
                98 | 99 => DdsFormat::Bc7,
                other => return Err(format!("DXGI 格式未入消费闭集: {other}")),
            }
        }
        other => {
            return Err(format!(
                "DDS FourCC 未入消费闭集: {}",
                String::from_utf8_lossy(other)
            ));
        }
    };
    let bb = format.block_bytes();
    let need = (width.div_ceil(4) as usize) * (height.div_ceil(4) as usize) * bb;
    let blocks = bytes.get(data_off..data_off + need).ok_or_else(|| {
        format!(
            "DDS 体截断: 需 {need} 字节(mip 0), 存 {}",
            bytes.len().saturating_sub(data_off)
        )
    })?;
    let rgba8 = match format {
        DdsFormat::Bc1 => decode_bc1_rgba8(blocks, width, height),
        DdsFormat::Bc3 => decode_bc3_rgba8(blocks, width, height),
        DdsFormat::Bc4 => decode_bc4_r8(blocks, width, height),
        DdsFormat::Bc5 => decode_bc5_rg8(blocks, width, height),
        DdsFormat::Bc7 => decode_bc7_rgba8(blocks, width, height),
    };
    Ok(DdsImage {
        width,
        height,
        format,
        mip_count,
        rgba8,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bc1_four_color_block_anchor() {
        // c0 = 纯红 0xF800,c1 = 纯蓝 0x001F(c0>c1 四色模式),全索引 0。
        let mut block = [0u8; 8];
        block[0..2].copy_from_slice(&0xF800u16.to_le_bytes());
        block[2..4].copy_from_slice(&0x001Fu16.to_le_bytes());
        let mut texels = [[0u8; 4]; 16];
        decode_bc1_block(&block, &mut texels);
        assert_eq!(texels[0], [255, 0, 0, 255]);
        // 索引 2 = (2·c0+c1)/3 = [170,0,85];索引 3 = (c0+2·c1)/3 = [85,0,170]
        // (通道级整数截断;蓝 0x001F → [0,0,255])。
        let mut b2 = block;
        b2[4] = 0b10; // texel0 索引=2
        decode_bc1_block(&b2, &mut texels);
        assert_eq!(texels[0], [170, 0, 85, 255]);
        b2[4] = 0b11; // texel0 索引=3
        decode_bc1_block(&b2, &mut texels);
        assert_eq!(texels[0], [85, 0, 170, 255]);
    }

    #[test]
    fn bc1_three_color_transparent_anchor() {
        // c0 <= c1:索引 2 = 平均色;索引 3 = 全 0(透明)。
        let mut block = [0u8; 8];
        block[0..2].copy_from_slice(&0x0000u16.to_le_bytes());
        block[2..4].copy_from_slice(&0xFFFFu16.to_le_bytes());
        block[4] = 0b1110_0100; // texel0 idx=0, texel1 idx=1, texel2 idx=2, texel3 idx=3
        let mut texels = [[0u8; 4]; 16];
        decode_bc1_block(&block, &mut texels);
        assert_eq!(texels[0], [0, 0, 0, 255]);
        assert_eq!(texels[1], [255, 255, 255, 255]);
        assert_eq!(texels[2], [127, 127, 127, 255]);
        assert_eq!(texels[3], [0, 0, 0, 0]);
    }

    #[test]
    fn bc3_alpha_from_bc4_lane() {
        // alpha e0=255 e1=0 全索引 0 → alpha=255;颜色 c0>c1 全索引 0 → c0。
        let mut block = [0u8; 16];
        block[0] = 255;
        block[1] = 0;
        block[8..10].copy_from_slice(&0x07E0u16.to_le_bytes()); // 纯绿
        block[10..12].copy_from_slice(&0x0000u16.to_le_bytes());
        let out = decode_bc3_rgba8(&block, 4, 4);
        assert_eq!(&out[0..4], &[0, 255, 0, 255]);
    }

    #[test]
    fn dds_container_fail_closed_and_legacy_parse() {
        assert!(decode_dds(b"not dds").is_err());
        // 构造最小 DXT1:4×4 单块(c0 红 > c1 蓝,全索引 0)。
        let mut dds = vec![0u8; 128 + 8];
        dds[0..4].copy_from_slice(b"DDS ");
        dds[4..8].copy_from_slice(&124u32.to_le_bytes());
        dds[12..16].copy_from_slice(&4u32.to_le_bytes());
        dds[16..20].copy_from_slice(&4u32.to_le_bytes());
        dds[28..32].copy_from_slice(&1u32.to_le_bytes());
        dds[76..80].copy_from_slice(&32u32.to_le_bytes());
        dds[84..88].copy_from_slice(b"DXT1");
        dds[128..130].copy_from_slice(&0xF800u16.to_le_bytes());
        dds[130..132].copy_from_slice(&0x001Fu16.to_le_bytes());
        let img = decode_dds(&dds).unwrap();
        assert_eq!((img.width, img.height, img.format), (4, 4, DdsFormat::Bc1));
        assert_eq!(&img.rgba8[0..4], &[255, 0, 0, 255]);
        // 体截断 fail-closed。
        assert!(decode_dds(&dds[..130]).is_err());
    }
}
