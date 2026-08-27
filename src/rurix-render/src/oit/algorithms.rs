//! OIT 七算法确定性参照实现（G9.5 M120；RFC-0025 §4.K；spec/display_pipeline.md
//! RXS-0371 L1/L4）。
//!
//! 以 nvpro `vk_order_independent_transparency` 七算法 sample 为对照基线
//! （同场景、同 overdraw 分布——[`super::scene::canonical_scene`] 单源生成），
//! host 确定性参照（禁 atomic 纪律承接 .rx kernel 面：atomics 以确定性串行
//! 语义模拟，真实 GPU 争用维度不进本波测量面,evidence 如实登记）。
//!
//! **统一合成约定**（七算法与排序真值同一口径,差异可归因）：
//! - kept 集按 (depth 降序 = 远→近, seq 升序 tie-break) 排序,over 累积
//!   （premult 形式 `kc = c·a + kc·(1-a)`, `ka = a + ka·(1-a)`）;
//! - tail 集按提交序 over 到不透明背景上（nvro color pass 的 tail-blend 流）;
//! - 最终 = kept 结果 over (tail over 背景) = `kc + t·(1-ka)`。
//!
//! 七算法（nvpro 存储/溢出语义逐字对应）：
//! | 算法 | 存储模型(bytes/px 或全局) | 保留策略 | 颜色量化 |
//! |---|---|---|---|
//! | simple | cap×8B + 4B 计数 | 提交序前 cap 个 | RGBA8 |
//! | linked_list | 4B head + 节点 24B 全局池 | 全保留(池界内) | 不量化(f32,精确档) |
//! | loop32 | cap×4B depth + cap×4B color(双 pass) | 最近 cap 个 | RGBA8 |
//! | loop64 | cap×8B(单 pass) | 最近 cap 个 | RGBA8 |
//! | spinlock | cap×8B + 计数/spin/depth 各 4B | 最近 cap(最远逐出) | RGBA8 |
//! | interlock | cap×8B + 4B 计数 | 最近 cap(插入排序) | RGBA8 |
//! | weighted | 8B accum + 2B reveal(O(1)) | 无保留(加权累积) | f32 累积 |
//!
//! 排序 fallback（depth-sorted alpha, [`sorted_fallback`]）**永远保留**为最低端
//! 档与正确性对照 = 排序真值本体;linked-list 精确档（仅毛发 strand 作用域）与
//! 真值 diff=0（同 fragment 集 + 同比较器 + 同合成,位级一致）。

use super::scene::{Fragment, OitScene};

/// 每像素固定层数(nvpro `OIT_LAYERS` 默认 8,冻结)。
pub const OIT_LAYERS: usize = 8;

/// 七算法闭集(nvpro 对照基线)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OitAlgorithm {
    Simple,
    LinkedList,
    Loop32,
    Loop64,
    Spinlock,
    Interlock,
    WeightedBlended,
}

impl OitAlgorithm {
    pub const ALL: [OitAlgorithm; 7] = [
        OitAlgorithm::Simple,
        OitAlgorithm::LinkedList,
        OitAlgorithm::Loop32,
        OitAlgorithm::Loop64,
        OitAlgorithm::Spinlock,
        OitAlgorithm::Interlock,
        OitAlgorithm::WeightedBlended,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            OitAlgorithm::Simple => "simple",
            OitAlgorithm::LinkedList => "linked_list",
            OitAlgorithm::Loop32 => "loop32",
            OitAlgorithm::Loop64 => "loop64",
            OitAlgorithm::Spinlock => "spinlock",
            OitAlgorithm::Interlock => "interlock",
            OitAlgorithm::WeightedBlended => "weighted_blended",
        }
    }
}

/// 算法测量产物。
#[derive(Debug, Clone)]
pub struct AlgoOutput {
    /// 最终帧 RGB(不含 alpha;背景上不透明)。
    pub rgb: Vec<[f32; 3]>,
    /// 存储模型 bytes(nvpro shader 资源布局公式化)。
    pub storage_bytes: u64,
    /// 辅助面 bytes(计数器/锁/头表等)。
    pub aux_bytes: u64,
    /// kept fragment 总数。
    pub fragments_kept: u64,
    /// tail-blend fragment 总数(溢出)。
    pub fragments_tail: u64,
    /// 池界外丢弃数(linked-list 池满;canonical 池充足 ⇒ 0)。
    pub fragments_dropped: u64,
}

/// RGBA8 量化(nvpro `packUnorm4x8(sRGB)` 语义:linear→sRGB→unorm8;alpha 线性)。
pub fn quantize_rgba8(c: [f32; 4]) -> [u8; 4] {
    let srgb = |v: f32| -> u8 {
        let v = v.clamp(0.0, 1.0);
        let e = if v <= 0.003_130_8 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (e * 255.0 + 0.5).floor() as u8
    };
    [
        srgb(c[0]),
        srgb(c[1]),
        srgb(c[2]),
        (c[3].clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8,
    ]
}

/// RGBA8 反量化(unpack + sRGB→linear)。
fn dequantize_rgba8(q: [u8; 4]) -> [f32; 4] {
    let lin = |v: u8| -> f32 {
        let e = v as f32 / 255.0;
        if e <= 0.040_45 {
            e / 12.92
        } else {
            ((e + 0.055) / 1.055).powf(2.4)
        }
    };
    [lin(q[0]), lin(q[1]), lin(q[2]), q[3] as f32 / 255.0]
}

/// 存储条目(量化后 color + depth + seq)。
#[derive(Clone, Copy)]
struct Entry {
    rgba8: [u8; 4],
    depth: f32,
    seq: u32,
}

/// over 累积(premult,straight-alpha 输入)。
fn over_acc(acc_c: &mut [f32; 3], acc_a: &mut f32, rgba: [f32; 4]) {
    let a = rgba[3].clamp(0.0, 1.0);
    for i in 0..3 {
        acc_c[i] = rgba[i] * a + acc_c[i] * (1.0 - a);
    }
    *acc_a = a + *acc_a * (1.0 - a);
}

/// kept 排序比较器(远→近 = depth 降序;seq 升序 tie-break;全算法单源)。
fn sort_kept(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        b.depth
            .partial_cmp(&a.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.seq.cmp(&b.seq))
    });
}

/// 统一合成:kept(远→近) over (tail 提交序 over 背景)。
fn composite(kept: &mut [Entry], tail: &[Entry], background: [f32; 3]) -> [f32; 3] {
    sort_kept(kept);
    let mut t = background;
    let mut ta = 1.0f32; // 背景不透明
    for e in tail {
        over_acc(&mut t, &mut ta, dequantize_rgba8(e.rgba8));
    }
    let mut kc = [0.0f32; 3];
    let mut ka = 0.0f32;
    for e in kept.iter() {
        over_acc(&mut kc, &mut ka, dequantize_rgba8(e.rgba8));
    }
    [
        kc[0] + t[0] * (1.0 - ka),
        kc[1] + t[1] * (1.0 - ka),
        kc[2] + t[2] * (1.0 - ka),
    ]
}

/// f32 精确合成(linked-list 与排序真值共享;不量化)。
fn composite_exact(frags: &mut [Fragment], background: [f32; 3]) -> [f32; 3] {
    frags.sort_by(|a, b| {
        b.depth
            .partial_cmp(&a.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.seq.cmp(&b.seq))
    });
    let mut kc = [0.0f32; 3];
    let mut ka = 0.0f32;
    for f in frags.iter() {
        over_acc(&mut kc, &mut ka, f.rgba);
    }
    [
        kc[0] + background[0] * (1.0 - ka),
        kc[1] + background[1] * (1.0 - ka),
        kc[2] + background[2] * (1.0 - ka),
    ]
}

fn to_entry(f: &Fragment) -> Entry {
    Entry {
        rgba8: quantize_rgba8(f.rgba),
        depth: f.depth,
        seq: f.seq,
    }
}

/// 排序 fallback / 排序真值(depth-sorted alpha;永保留最低端档与正确性对照)。
pub fn sorted_fallback(scene: &OitScene) -> AlgoOutput {
    let mut rgb = vec![[0.0f32; 3]; scene.pixel_count()];
    let mut kept_total = 0u64;
    for (p, px) in rgb.iter_mut().enumerate() {
        let mut frags: Vec<Fragment> = scene.pixel_fragments(p).to_vec();
        kept_total += frags.len() as u64;
        *px = composite_exact(&mut frags, scene.background);
    }
    AlgoOutput {
        rgb,
        storage_bytes: kept_total * 24, // f32×4 + depth + seq canonical 24B
        aux_bytes: scene.pixel_count() as u64 * 4,
        fragments_kept: kept_total,
        fragments_tail: 0,
        fragments_dropped: 0,
    }
}

/// Simple(nvpro `oitSimple`:提交序前 cap 存入,溢出 tail-blend;resolve 排序)。
fn run_simple(scene: &OitScene) -> AlgoOutput {
    let cap = OIT_LAYERS;
    let mut rgb = vec![[0.0f32; 3]; scene.pixel_count()];
    let (mut kept_n, mut tail_n) = (0u64, 0u64);
    for (p, px) in rgb.iter_mut().enumerate() {
        let frags = scene.pixel_fragments(p);
        let kept_count = frags.len().min(cap);
        let mut kept: Vec<Entry> = frags.iter().take(kept_count).map(to_entry).collect();
        let tail: Vec<Entry> = frags.iter().skip(kept_count).map(to_entry).collect();
        kept_n += kept.len() as u64;
        tail_n += tail.len() as u64;
        *px = composite(&mut kept, &tail, scene.background);
    }
    let pix = scene.pixel_count() as u64;
    AlgoOutput {
        rgb,
        storage_bytes: pix * cap as u64 * 8, // u32 packed color + u32 depth
        aux_bytes: pix * 4,                  // counter
        fragments_kept: kept_n,
        fragments_tail: tail_n,
        fragments_dropped: 0,
    }
}

/// LinkedList(nvpro `oitLinkedList` 语义 + RXS-0371 精确档:f32 不量化,
/// 全局节点池;池内全保留 ⇒ 与排序真值 diff=0;池满丢弃计数)。
fn run_linked_list(scene: &OitScene, pool_cap_nodes: u64) -> AlgoOutput {
    let mut rgb = vec![[0.0f32; 3]; scene.pixel_count()];
    let (mut kept_n, mut dropped) = (0u64, 0u64);
    let mut used = 0u64;
    for (p, px) in rgb.iter_mut().enumerate() {
        let frags = scene.pixel_fragments(p);
        let avail = pool_cap_nodes.saturating_sub(used);
        let take = (frags.len() as u64).min(avail) as usize;
        let mut kept: Vec<Fragment> = frags.iter().take(take).copied().collect();
        used += take as u64;
        kept_n += take as u64;
        dropped += (frags.len() - take) as u64;
        *px = composite_exact(&mut kept, scene.background);
    }
    let pix = scene.pixel_count() as u64;
    AlgoOutput {
        rgb,
        storage_bytes: used * 24, // f32 color×4 + f32 depth + u32 next(+pad)=24B 节点
        aux_bytes: pix * 4 + 4,   // head 表 + 全局计数
        fragments_kept: kept_n,
        fragments_tail: 0,
        fragments_dropped: dropped,
    }
}

/// Loop32(nvpro `oitLoop`:depth pass 保最近 cap 个 + color pass 配对;双 pass)。
/// Loop64(nvpro `oitLoop64`:u64 条目单 pass;保留集相同,存储布局不同)。
fn run_loop(scene: &OitScene, is64: bool) -> AlgoOutput {
    let cap = OIT_LAYERS;
    let mut rgb = vec![[0.0f32; 3]; scene.pixel_count()];
    let (mut kept_n, mut tail_n) = (0u64, 0u64);
    for (p, px) in rgb.iter_mut().enumerate() {
        let frags = scene.pixel_fragments(p);
        // 保最近 cap 个(depth 升序取前 cap ⇒ 最近集)。
        let mut order: Vec<usize> = (0..frags.len()).collect();
        order.sort_by(|&a, &b| {
            frags[a]
                .depth
                .partial_cmp(&frags[b].depth)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(frags[a].seq.cmp(&frags[b].seq))
        });
        let keep_idx: std::collections::BTreeSet<usize> = order.iter().take(cap).copied().collect();
        let mut kept: Vec<Entry> = Vec::with_capacity(cap);
        let mut tail: Vec<Entry> = Vec::new();
        for (i, f) in frags.iter().enumerate() {
            if keep_idx.contains(&i) {
                kept.push(to_entry(f));
            } else {
                tail.push(to_entry(f));
            }
        }
        kept_n += kept.len() as u64;
        tail_n += tail.len() as u64;
        *px = composite(&mut kept, &tail, scene.background);
    }
    let pix = scene.pixel_count() as u64;
    let storage = if is64 {
        pix * cap as u64 * 8 // u64 条目(depth32+color32)
    } else {
        pix * cap as u64 * 4 + pix * cap as u64 * 4 // depth 区 + color 区(双 buffer)
    };
    AlgoOutput {
        rgb,
        storage_bytes: storage,
        aux_bytes: 0,
        fragments_kept: kept_n,
        fragments_tail: tail_n,
        fragments_dropped: 0,
    }
}

/// Spinlock/Interlock(nvpro 同语义:固定数组 + 最远逐出保最近 cap;
/// spinlock 多 spin/depth 缓存字,interlock 硬件临界区——host 模型同保留集,
/// 机制差异进元数据)。
fn run_spinlock(scene: &OitScene, interlock: bool) -> AlgoOutput {
    let cap = OIT_LAYERS;
    let mut rgb = vec![[0.0f32; 3]; scene.pixel_count()];
    let (mut kept_n, mut tail_n) = (0u64, 0u64);
    for (p, px) in rgb.iter_mut().enumerate() {
        let frags = scene.pixel_fragments(p);
        let mut arr: Vec<Entry> = Vec::with_capacity(cap);
        let mut tail: Vec<Entry> = Vec::new();
        for f in frags {
            let e = to_entry(f);
            if arr.len() < cap {
                arr.push(e);
            } else {
                // 找最远逐出(nvpro 逐字策略)。
                let (furthest_i, furthest) = arr
                    .iter()
                    .enumerate()
                    .max_by(|a, b| {
                        a.1.depth
                            .partial_cmp(&b.1.depth)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .expect("cap>0");
                let (fd, fi) = (furthest.depth, furthest_i);
                if e.depth < fd {
                    let evicted = arr[fi];
                    arr[fi] = e;
                    tail.push(evicted);
                } else {
                    tail.push(e);
                }
            }
        }
        kept_n += arr.len() as u64;
        tail_n += tail.len() as u64;
        *px = composite(&mut arr, &tail, scene.background);
    }
    let pix = scene.pixel_count() as u64;
    let aux = if interlock {
        pix * 4 // counter(硬件 interlock 无 spin 字)
    } else {
        pix * 4 + pix * 4 + pix * 4 // counter + spin + depth 缓存
    };
    AlgoOutput {
        rgb,
        storage_bytes: pix * cap as u64 * 8,
        aux_bytes: aux,
        fragments_kept: kept_n,
        fragments_tail: tail_n,
        fragments_dropped: 0,
    }
}

/// Weighted Blended(nvpro `oitWeighted` 逐字权重;单 pass O(1) 内存,近似档)。
fn run_weighted(scene: &OitScene) -> AlgoOutput {
    let mut rgb = vec![[0.0f32; 3]; scene.pixel_count()];
    let mut kept_n = 0u64;
    for (p, px) in rgb.iter_mut().enumerate() {
        let frags = scene.pixel_fragments(p);
        kept_n += frags.len() as u64;
        let mut accum = [0.0f32; 4];
        let mut reveal = 1.0f32;
        for f in frags {
            // nvpro 权重:distWeight = clamp(0.03/(1e-5 + (depthZ/200)^4), 1e-2, 3e3),
            // depthZ = -viewZ·10;alphaWeight = min(1, max(r,g,b,a)·40 + 0.01)²。
            let depth_z = f.depth * 10.0;
            let dist_weight = (0.03f32 / (1e-5 + (depth_z / 200.0).powi(4))).clamp(1e-2, 3e3);
            let premult = [
                f.rgba[0] * f.rgba[3],
                f.rgba[1] * f.rgba[3],
                f.rgba[2] * f.rgba[3],
            ];
            let alpha_weight = (1.0f32
                .min(premult[0].max(premult[1]).max(premult[2]).max(f.rgba[3]) * 40.0 + 0.01))
            .powi(2);
            let w = alpha_weight * dist_weight;
            for i in 0..3 {
                accum[i] += premult[i] * w;
            }
            accum[3] += f.rgba[3] * w;
            reveal *= 1.0 - f.rgba[3];
        }
        // resolve:color = accum.rgb / max(accum.a, 1e-5);over 背景 with (1-reveal)。
        let avg_a = accum[3].max(1e-5);
        for i in 0..3 {
            let c = accum[i] / avg_a;
            px[i] = c * (1.0 - reveal) + scene.background[i] * reveal;
        }
    }
    let pix = scene.pixel_count() as u64;
    AlgoOutput {
        rgb,
        storage_bytes: pix * 8 + pix * 2, // RGBA16F accum + R16F reveal
        aux_bytes: 0,
        fragments_kept: kept_n,
        fragments_tail: 0,
        fragments_dropped: 0,
    }
}

/// linked-list 精确档默认池界(canonical 场景总 fragment 数;充足 ⇒ diff=0)。
pub fn exact_tier_pool_cap(scene: &OitScene) -> u64 {
    scene.stream.len() as u64
}

/// 七算法分发(cap/池默认面;harness 经 [`run_algorithm`] 消费)。
pub fn run_algorithm(algo: OitAlgorithm, scene: &OitScene) -> AlgoOutput {
    match algo {
        OitAlgorithm::Simple => run_simple(scene),
        OitAlgorithm::LinkedList => run_linked_list(scene, exact_tier_pool_cap(scene)),
        OitAlgorithm::Loop32 => run_loop(scene, false),
        OitAlgorithm::Loop64 => run_loop(scene, true),
        OitAlgorithm::Spinlock => run_spinlock(scene, false),
        OitAlgorithm::Interlock => run_spinlock(scene, true),
        OitAlgorithm::WeightedBlended => run_weighted(scene),
    }
}

/// 图像 digest(f32 LE)。
pub fn image_digest(rgb: &[[f32; 3]]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(rgb.len() * 12);
    for p in rgb {
        for c in p {
            buf.extend_from_slice(&c.to_le_bytes());
        }
    }
    rurix_pkg::sha256::digest(&buf)
}

/// 质量误差面(对排序真值):max/mean 绝对差 + 超阈像素计数。
pub fn quality_error(out: &[[f32; 3]], truth: &[[f32; 3]], eps: f32) -> (f32, f64, u32) {
    let mut max_abs = 0.0f32;
    let mut sum = 0.0f64;
    let mut count = 0u32;
    for (a, b) in out.iter().zip(truth.iter()) {
        let d = (a[0] - b[0])
            .abs()
            .max((a[1] - b[1]).abs())
            .max((a[2] - b[2]).abs());
        max_abs = max_abs.max(d);
        sum += d as f64;
        if d > eps {
            count += 1;
        }
    }
    let mean = if out.is_empty() {
        0.0
    } else {
        sum / out.len() as f64
    };
    (max_abs, mean, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oit::scene::canonical_scene;

    //@ spec: RXS-0371
    #[test]
    fn linked_list_exact_vs_sorted_truth_diff_zero() {
        // RXS-0371 L4:linked-list 精确档与排序真值 diff=0(池充足)。
        let scene = canonical_scene(32, 32, 64);
        let truth = sorted_fallback(&scene);
        let ll = run_algorithm(OitAlgorithm::LinkedList, &scene);
        assert_eq!(ll.fragments_dropped, 0, "canonical 池充足");
        let (max_abs, _, over0) = quality_error(&ll.rgb, &truth.rgb, 0.0);
        assert_eq!(max_abs, 0.0);
        assert_eq!(over0, 0, "精确档与真值位级一致");
        assert_eq!(image_digest(&ll.rgb), image_digest(&truth.rgb));
    }

    //@ spec: RXS-0371
    #[test]
    fn sorted_fallback_always_available() {
        // RXS-0371 L4:排序 fallback 永保留(可达断言)。
        let scene = canonical_scene(16, 16, 8);
        let out = sorted_fallback(&scene);
        assert_eq!(out.rgb.len(), scene.pixel_count());
        assert!(out.fragments_kept > 0);
    }

    //@ spec: RXS-0371
    #[test]
    fn approximate_algorithms_show_measured_error() {
        // 深 overdraw 下近似档必须可见误差(测量敏感性),浅层近似真值。
        let deep = canonical_scene(32, 32, 256);
        let truth = sorted_fallback(&deep);
        let w = run_algorithm(OitAlgorithm::WeightedBlended, &deep);
        let (max_abs, _, count) = quality_error(&w.rgb, &truth.rgb, 1e-4);
        assert!(
            max_abs > 1e-3 && count > 0,
            "WBOIT 近似误差可测: max={max_abs} n={count}"
        );
        let s = run_algorithm(OitAlgorithm::Simple, &deep);
        let (smax, _, scount) = quality_error(&s.rgb, &truth.rgb, 1e-4);
        assert!(smax > 0.0 && scount > 0, "simple 溢出误差可测: {smax}");
        assert!(s.fragments_tail > 0, "深场景溢出非空");
    }

    //@ spec: RXS-0371
    #[test]
    fn memory_models_per_algorithm_formula() {
        let scene = canonical_scene(32, 32, 64);
        let pix = 32u64 * 32;
        let s = run_algorithm(OitAlgorithm::Simple, &scene);
        assert_eq!(s.storage_bytes, pix * 8 * 8);
        let w = run_algorithm(OitAlgorithm::WeightedBlended, &scene);
        assert_eq!(w.storage_bytes, pix * 10); // O(1):与 overdraw 无关
        let scene2 = canonical_scene(32, 32, 256);
        let w2 = run_algorithm(OitAlgorithm::WeightedBlended, &scene2);
        assert_eq!(w2.storage_bytes, w.storage_bytes);
        let ll = run_algorithm(OitAlgorithm::LinkedList, &scene2);
        let ll1 = run_algorithm(OitAlgorithm::LinkedList, &scene);
        assert!(
            ll.storage_bytes > ll1.storage_bytes,
            "linked-list 内存随 fragment 增长"
        );
    }
}
