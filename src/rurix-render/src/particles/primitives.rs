//! G35-1 24 位键 3-pass 稳定 LSD radix sort + compact_u32 基元——host 金标准
//! (门 `g35.wave1.primitives`;RFC-0049 §4.2;契约锚 = mod.rs G35-P v1)。
//!
//! 与 device 四 kernel(`kernels/g35_sort_hist.rx` / `g35_sort_spine.rx` /
//! `g35_sort_scatter.rx` / `g35_compact_u32.rx`)**逐字同源**的镜像分解:纯
//! u32/usize 整数算术、零浮点、固定迭代序 ⇒ device↔host **零容差位级**对拍
//! (mod.rs 整数域协议)。确定性协议第一条(禁原子抢槽)的排序面兑现:
//! 每 pass = 段直方图([`sort_hist`])→ digit-major 单串行 exclusive scan
//! ([`sort_spine`])→ 段内串行稳定散射([`sort_scatter`]),稳定序 =
//! 段序 × 段内下标序,与线程调度无关 ⇒ 固定输入双跑位级一致。digit =
//! `(key / dpow) % 256`,dpow ∈ {1, 256, 65536}(语言面无位运算,usize
//! 除/模精确算术,g34_unified_gi.rx 图集 unpack 先例);Onesweep/
//! decoupled-lookback 评估窗登记不实现(Vulkan 前进保证缺位,保守分段臂 =
//! 生产形态,RFC-0049 §9 Q1)。[`compact_u32`] 消费
//! [`super::scan::exclusive_scan_segmented`](scan 三 kernel host 同源)。
//!
//! 域前提:keys < 2^24([`sort_pairs_u24`] debug_assert)、n ≤ SEG·NSEG_MAX、
//! flags ∈ {0,1}(调用方保证;深度键 [`super::depth_key24`] 产 / 存活旗标
//! 面天然满足)。

use super::SEG;

/// 阶段 1(= g35_sort_hist.rx):线程 s 段内串行数字直方图。
/// 返回长度 nseg·256:hist[s·256 + d] = 段 s 内 digit d 计数,
/// d = (key / dpow) % 256。段外线程界外守卫直落(kernel 同律:行清零 +
/// 零次计数循环 = 全零行,与本函数零初始化同值)。
pub fn sort_hist(keys: &[u32], nseg: usize, dpow: usize) -> Vec<u32> {
    let n = keys.len();
    let mut hist = vec![0u32; nseg * 256];
    for s in 0..nseg {
        let lo = s * SEG;
        let hi = ((s + 1) * SEG).min(n);
        if lo < n {
            for i in lo..hi {
                let d = (keys[i] as usize / dpow) % 256;
                hist[s * 256 + d] += 1;
            }
        }
    }
    hist
}

/// 阶段 2(= g35_sort_spine.rx):digit-major 单串行 exclusive scan。
/// 返回长度 256·nseg:off[d·nseg + s] = digit d、段 s 的稳定散射基址
/// (digit 升序 × 段升序 = LSD radix 稳定序之源;单串行 = 确定性天然全序)。
pub fn sort_spine(hist: &[u32], nseg: usize) -> Vec<u32> {
    let mut off = vec![0u32; 256 * nseg];
    let mut acc: u32 = 0;
    for d in 0..256 {
        for s in 0..nseg {
            off[d * nseg + s] = acc;
            acc += hist[s * 256 + d];
        }
    }
    off
}

/// 阶段 3(= g35_sort_scatter.rx):线程 s 镜像 = 段循环,段内串行稳定散射。
/// scratch(长度 nseg·256,函数内逐段行先清零——kernel 行清零逐字同源)为
/// 段内 running 计数;idx = off[d·nseg + s] + scratch[s·256 + d],双流散射。
pub fn sort_scatter(
    keys: &[u32],
    payload: &[u32],
    scratch: &mut [u32],
    off: &[u32],
    nseg: usize,
    dpow: usize,
) -> (Vec<u32>, Vec<u32>) {
    let n = keys.len();
    let mut keys_out = vec![0u32; n];
    let mut payload_out = vec![0u32; n];
    for s in 0..nseg {
        for d in 0..256 {
            scratch[s * 256 + d] = 0;
        }
        let lo = s * SEG;
        let hi = ((s + 1) * SEG).min(n);
        if lo < n {
            for i in lo..hi {
                let d = (keys[i] as usize / dpow) % 256;
                let idx = (off[d * nseg + s] + scratch[s * 256 + d]) as usize;
                scratch[s * 256 + d] += 1;
                keys_out[idx] = keys[i];
                payload_out[idx] = payload[i];
            }
        }
    }
    (keys_out, payload_out)
}

/// 三阶段 × 3 pass 串联(dpow = 1 / 256 / 65536):24 位键稳定 LSD radix
/// sort(= 三 kernel 全链的 host 镜像;pass 间 ping-pong 由本函数 Vec 交接
/// 承载,device 面 = probe bin host 侧缓冲交换同律)。
pub fn sort_pairs_u24(keys: &[u32], payload: &[u32]) -> (Vec<u32>, Vec<u32>) {
    assert_eq!(keys.len(), payload.len(), "keys/payload 长度必须一致");
    debug_assert!(
        keys.iter().all(|&k| k < 16_777_216),
        "键域断言:keys < 2^24(depth_key24 产键面契约)"
    );
    let nseg = keys.len().div_ceil(SEG);
    let mut k = keys.to_vec();
    let mut p = payload.to_vec();
    let mut scratch = vec![0u32; nseg * 256];
    for dpow in [1usize, 256, 65536] {
        let hist = sort_hist(&k, nseg, dpow);
        let off = sort_spine(&hist, nseg);
        let (k2, p2) = sort_scatter(&k, &p, &mut scratch, &off, nseg, dpow);
        k = k2;
        p = p2;
    }
    (k, p)
}

/// flags 压缩基元(= g35_compact_u32.rx 的 host 镜像):消费
/// [`super::scan::exclusive_scan_segmented`](scan 三 kernel host 同源)推导
/// 槽位,out[scan[i]] = values[i](flags[i] ≠ 0),顺序 = 元素下标序
/// (禁原子抢槽协议的压缩面兑现)。返回长度 = Σflags。
pub fn compact_u32(values: &[u32], flags: &[u32]) -> Vec<u32> {
    assert_eq!(values.len(), flags.len(), "values/flags 长度必须一致");
    debug_assert!(
        flags.iter().all(|&f| f <= 1),
        "flags 域断言:flags ∈ {{0,1}}(存活旗标面契约)"
    );
    let (scan, total) = super::scan::exclusive_scan_segmented(flags);
    let mut out = vec![0u32; total as usize];
    for i in 0..values.len() {
        if flags[i] != 0 {
            out[scan[i] as usize] = values[i];
        }
    }
    out
}

/// 独立参考实现(std 稳定 sort_by_key 直书)——与分段分解互核用,防"同一
/// 错误两处照抄"(scan.rs::exclusive_scan_reference 同律)。
pub fn sort_pairs_reference(keys: &[u32], payload: &[u32]) -> (Vec<u32>, Vec<u32>) {
    assert_eq!(keys.len(), payload.len(), "keys/payload 长度必须一致");
    let mut pairs: Vec<(u32, u32)> = keys.iter().copied().zip(payload.iter().copied()).collect();
    pairs.sort_by_key(|&(k, _)| k);
    (
        pairs.iter().map(|&(k, _)| k).collect(),
        pairs.iter().map(|&(_, p)| p).collect(),
    )
}

/// 独立参考实现(iter filter 直书)——与 scan 分解压缩互核用。
pub fn compact_reference(values: &[u32], flags: &[u32]) -> Vec<u32> {
    assert_eq!(values.len(), flags.len(), "values/flags 长度必须一致");
    values
        .iter()
        .zip(flags.iter())
        .filter(|&(_, &f)| f != 0)
        .map(|(&v, _)| v)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::{Pcg32, SEG};
    use super::*;

    fn fixture(n: usize, seed: u64, modulo: u32) -> (Vec<u32>, Vec<u32>) {
        let mut rng = Pcg32::new(seed, 54);
        let keys: Vec<u32> = (0..n).map(|_| rng.next_u32() % modulo).collect();
        let payload: Vec<u32> = (0..n as u32).collect();
        (keys, payload)
    }

    #[test]
    fn sort_matches_reference_on_random_scales() {
        for &n in &[0usize, 1, SEG - 1, SEG, SEG + 1, 4096, 65536] {
            let (keys, payload) = fixture(n, 42, 16_777_216);
            let (sk, sp) = sort_pairs_u24(&keys, &payload);
            let (rk, rp) = sort_pairs_reference(&keys, &payload);
            assert_eq!(sk, rk, "n={n} 分段分解键序 ≠ 参考实现");
            assert_eq!(sp, rp, "n={n} 分段分解 payload ≠ 参考实现(稳定性破)");
        }
    }

    #[test]
    fn sort_stable_on_duplicate_keys() {
        // 重复键域(%16 ⇒ 段内段间大量同键):payload = 原下标,稳定 ⇔ 同键段
        // payload 严格递增;与 std 稳定 sort 参考互核双保险。
        let (keys, payload) = fixture(4096, 7, 16);
        let (sk, sp) = sort_pairs_u24(&keys, &payload);
        let (rk, rp) = sort_pairs_reference(&keys, &payload);
        assert_eq!(sk, rk);
        assert_eq!(sp, rp);
        let mut dup_pairs = 0usize;
        for w in 1..sk.len() {
            if sk[w] == sk[w - 1] {
                dup_pairs += 1;
                assert!(sp[w] > sp[w - 1], "同键 payload 逆序:稳定性破(w={w})");
            }
        }
        assert!(dup_pairs > 0, "夹具必须含重复键(稳定性判据咬合前提)");
    }

    #[test]
    fn sort_edge_domains() {
        // 全同键:排序 = 恒等(稳定性极限形)。
        let keys = vec![12_345u32; 700];
        let payload: Vec<u32> = (0..700).collect();
        let (sk, sp) = sort_pairs_u24(&keys, &payload);
        assert_eq!(sk, keys, "全同键键序必须恒等");
        assert_eq!(sp, payload, "全同键 payload 必须保原序");
        // 全零键。
        let keys = vec![0u32; 513];
        let payload: Vec<u32> = (0..513).collect();
        let (sk, sp) = sort_pairs_u24(&keys, &payload);
        assert_eq!(sk, keys);
        assert_eq!(sp, payload);
        // 严格递减序(最坏逆序;含 2^24−1 顶格键):对参考互核 + 单调断言。
        let keys: Vec<u32> = (0..1000u32).map(|i| 16_777_215 - i * 977).collect();
        let payload: Vec<u32> = (0..1000).collect();
        let (sk, sp) = sort_pairs_u24(&keys, &payload);
        let (rk, rp) = sort_pairs_reference(&keys, &payload);
        assert_eq!(sk, rk);
        assert_eq!(sp, rp);
        assert!(sk.windows(2).all(|w| w[0] <= w[1]), "输出必须单调不减");
        assert_eq!(sk[999], 16_777_215, "顶格键必须落尾");
    }

    #[test]
    fn compact_matches_reference_on_random_flags() {
        for &n in &[0usize, 1, SEG - 1, SEG, SEG + 1, 4096, 65536] {
            let mut rng = Pcg32::new(9, 54);
            let values: Vec<u32> = (0..n).map(|_| rng.next_u32() % 16_777_216).collect();
            let flags: Vec<u32> = (0..n).map(|_| rng.next_u32() % 2).collect();
            assert_eq!(
                compact_u32(&values, &flags),
                compact_reference(&values, &flags),
                "n={n} scan 分解压缩 ≠ 参考实现"
            );
        }
    }

    #[test]
    fn compact_edge_flags() {
        // 全 0 flags:空输出;全 1 flags:恒等(压缩边界双极)。
        let values: Vec<u32> = (0..700u32).collect();
        let zeros = vec![0u32; 700];
        let ones = vec![1u32; 700];
        assert_eq!(compact_u32(&values, &zeros), Vec::<u32>::new());
        assert_eq!(compact_u32(&values, &ones), values);
    }
}
