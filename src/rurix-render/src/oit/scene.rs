//! OIT benchmark 确定性场景面（G9.5 M120；RFC-0025 §4.K；spec/display_pipeline.md
//! RXS-0371 L1）。
//!
//! **同场景、同 overdraw 分布**：canonical 场景由整数 hash 闭式生成（零浮点
//! RNG），七算法共用同一份逐像素 fragment 流（digest 锚定）：
//! - 覆盖：pixel(x,y) × layer i 经 u32 混合 hash，~75% 覆盖（`h % 4 != 3`），
//!   逐像素 overdraw 呈二项分布（非平凡分布面）；
//! - 深度：view_z = 0.1 + (i + jitter)·49.9/L，jitter ∈ [0,0.9) 由 hash 驱动
//!   （提交序 = layer 升序，深度乱序 ⇒ 排序真值非平凡）；
//! - 颜色：RGB/α 由 hash 派生（straight alpha，α ∈ [0.15, 0.70]）。
//!
//! 全部 wrapping 整数运算,跨平台位级确定;digest 经 rurix-pkg SHA-256。

/// 半透明 fragment（straight alpha;canonical 字节 = rgba f32 LE ×4 + depth f32
/// LE + seq u32 LE）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fragment {
    pub rgba: [f32; 4],
    /// view 空间深度(z > 0,大 = 远)。
    pub depth: f32,
    /// 提交序(layer 序;排序同深度 tie-break)。
    pub seq: u32,
}

impl Fragment {
    /// canonical 字节(24B)。
    pub fn canonical_bytes(&self) -> [u8; 24] {
        let mut b = [0u8; 24];
        for (i, c) in self.rgba.iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&c.to_le_bytes());
        }
        b[16..20].copy_from_slice(&self.depth.to_le_bytes());
        b[20..24].copy_from_slice(&self.seq.to_le_bytes());
        b
    }
}

/// canonical 场景（逐像素 fragment 流 + 偏移表）。
#[derive(Debug, Clone)]
pub struct OitScene {
    pub width: u32,
    pub height: u32,
    pub layers: u32,
    /// 不透明背景(over 基底)。
    pub background: [f32; 3],
    /// 扁平 fragment 流(像素行主序拼接)。
    pub stream: Vec<Fragment>,
    /// 每像素起始偏移(长度 w*h+1)。
    pub offsets: Vec<u32>,
}

impl OitScene {
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// 像素 fragment 切片。
    pub fn pixel_fragments(&self, px: usize) -> &[Fragment] {
        &self.stream[self.offsets[px] as usize..self.offsets[px + 1] as usize]
    }

    /// 流 digest(场景锚定)。
    pub fn digest(&self) -> [u8; 32] {
        let mut buf = Vec::with_capacity(self.stream.len() * 24 + 32);
        buf.extend_from_slice(&self.width.to_le_bytes());
        buf.extend_from_slice(&self.height.to_le_bytes());
        buf.extend_from_slice(&self.layers.to_le_bytes());
        for c in self.background {
            buf.extend_from_slice(&c.to_le_bytes());
        }
        for f in &self.stream {
            buf.extend_from_slice(&f.canonical_bytes());
        }
        rurix_pkg::sha256::digest(&buf)
    }
}

/// u32 混合 hash(splitmix 族 wrapping 运算,位级确定)。
fn mix32(mut h: u32) -> u32 {
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}

fn hash3(x: u32, y: u32, i: u32) -> u32 {
    mix32(x.wrapping_mul(0x8da6_b343) ^ y.wrapping_mul(0xd816_3841) ^ i.wrapping_mul(0xcb1a_b31f))
}

/// 生成 canonical 场景(同场景同 overdraw 分布面)。
pub fn canonical_scene(width: u32, height: u32, layers: u32) -> OitScene {
    let background = [0.05f32, 0.07, 0.09];
    let pixels = (width * height) as usize;
    let mut stream = Vec::new();
    let mut offsets = Vec::with_capacity(pixels + 1);
    offsets.push(0u32);
    for py in 0..height {
        for px in 0..width {
            for i in 0..layers {
                let h = hash3(px, py, i);
                if h % 4 == 3 {
                    continue; // ~25% 空洞
                }
                let jitter = ((h >> 8) & 0x3ff) as f32 / 1024.0 * 0.9;
                let depth = 0.1 + (i as f32 + jitter) * (49.9 / layers.max(1) as f32);
                let h2 = mix32(h ^ 0x9e37_79b9);
                let r = ((h2 & 0x3ff) as f32 / 1023.0).powi(2) * 0.9 + 0.1;
                let g = (((h2 >> 10) & 0x3ff) as f32 / 1023.0).powi(2) * 0.9 + 0.1;
                let b = (((h2 >> 20) & 0x3ff) as f32 / 1023.0).powi(2) * 0.9 + 0.1;
                let alpha = 0.15 + 0.55 * (((h >> 20) & 0xff) as f32 / 255.0);
                stream.push(Fragment {
                    rgba: [r, g, b, alpha],
                    depth,
                    seq: i,
                });
            }
            offsets.push(stream.len() as u32);
        }
    }
    OitScene {
        width,
        height,
        layers,
        background,
        stream,
        offsets,
    }
}

/// benchmark 档位阶梯(overdraw 层数;曲线横轴,冻结闭集)。
pub const BENCHMARK_LAYERS: [u32; 4] = [4, 16, 64, 256];
/// benchmark 分辨率(冻结)。
pub const BENCHMARK_EXTENT: (u32, u32) = (128, 128);

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0371
    #[test]
    fn scene_deterministic_and_nontrivial() {
        let a = canonical_scene(32, 32, 16);
        let b = canonical_scene(32, 32, 16);
        assert_eq!(a.digest(), b.digest());
        // 非平凡:fragment 非空、逐像素计数有分布(非全同)、深度乱序存在。
        assert!(!a.stream.is_empty());
        let counts: Vec<usize> = (0..a.pixel_count())
            .map(|p| a.pixel_fragments(p).len())
            .collect();
        let min = *counts.iter().min().unwrap();
        let max = *counts.iter().max().unwrap();
        assert!(max > min, "overdraw 分布非平凡: min={min} max={max}");
        let mut inversions = 0u32;
        for p in 0..a.pixel_count() {
            let fr = a.pixel_fragments(p);
            for w in fr.windows(2) {
                if w[0].depth < w[1].depth {
                    inversions += 1;
                }
            }
        }
        assert!(inversions > 0, "提交序非深度序(排序真值非平凡)");
        // 层数缩放:更深场景 fragment 更多。
        let deep = canonical_scene(32, 32, 64);
        assert!(deep.stream.len() > a.stream.len());
    }
}
