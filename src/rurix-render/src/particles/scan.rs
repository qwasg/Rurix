//! G35-P 分段稳定 exclusive scan——host 金标准(冻结面;RFC-0049 §4.1)。
//!
//! 与 device 三 kernel(`kernels/g35_scan_seg_sum.rx` / `g35_scan_spine.rx` /
//! `g35_scan_seg_apply.rx`)**逐字同源**的三阶段分解:纯 u32/usize 整数
//! 算术、零浮点、固定迭代序 ⇒ device↔host **零容差位级**对拍(整数域协议,
//! mod.rs 契约)。稳定压缩/发射槽位/排序 spine 一律消费本面,禁原子抢槽
//! (确定性协议第一条)。
//!
//! 域前提:`values.len() = n ≤ SEG·nseg`,单值 < 2^24,总和 < 2^32
//! (调用方保证;粒子面 flags ∈ {0,1} 与 256 bin 直方图天然满足)。

use super::SEG;

/// 阶段 1(= g35_scan_seg_sum.rx):线程 s 串行求段和。
/// 返回 seg_sums[s] = Σ values[s·SEG .. min((s+1)·SEG, n)]。
pub fn seg_sums(values: &[u32], nseg: usize) -> Vec<u32> {
    let n = values.len();
    (0..nseg)
        .map(|s| {
            let lo = s * SEG;
            let hi = ((s + 1) * SEG).min(n);
            let mut acc: u32 = 0;
            if lo < n {
                for &v in &values[lo..hi] {
                    acc += v;
                }
            }
            acc
        })
        .collect()
}

/// 阶段 2(= g35_scan_spine.rx):单串行 exclusive scan。
/// 返回长度 nseg+1:`[0..nseg)` = 段基址,`[nseg]` = 总和(存活总数槽,
/// indirect args 消费面)。
pub fn spine(seg_sums: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(seg_sums.len() + 1);
    let mut acc: u32 = 0;
    for &s in seg_sums {
        out.push(acc);
        acc += s;
    }
    out.push(acc);
    out
}

/// 阶段 3(= g35_scan_seg_apply.rx):段内串行 running 前缀。
/// 返回全局 exclusive scan(长度 = values.len())。
pub fn seg_apply(values: &[u32], spine: &[u32], nseg: usize) -> Vec<u32> {
    let n = values.len();
    let mut out = vec![0u32; n];
    for s in 0..nseg {
        let lo = s * SEG;
        if lo >= n {
            break;
        }
        let hi = ((s + 1) * SEG).min(n);
        let mut running = spine[s];
        for i in lo..hi {
            out[i] = running;
            running += values[i];
        }
    }
    out
}

/// 三阶段合成:全局 exclusive scan + 总和(= 三 kernel 串联的 host 镜像)。
pub fn exclusive_scan_segmented(values: &[u32]) -> (Vec<u32>, u32) {
    let nseg = values.len().div_ceil(SEG);
    let sums = seg_sums(values, nseg);
    let sp = spine(&sums);
    let total = sp[nseg];
    (seg_apply(values, &sp, nseg), total)
}

/// 独立参考实现(单循环直书)——与分段分解互核用,防"同一错误两处照抄"。
pub fn exclusive_scan_reference(values: &[u32]) -> (Vec<u32>, u32) {
    let mut out = Vec::with_capacity(values.len());
    let mut acc: u32 = 0;
    for &v in values {
        out.push(acc);
        acc += v;
    }
    (out, acc)
}

#[cfg(test)]
mod tests {
    use super::super::{Pcg32, SEG};
    use super::*;

    #[test]
    fn segmented_matches_reference_on_random_flags() {
        let mut rng = Pcg32::new(9, 54);
        for &n in &[0usize, 1, SEG - 1, SEG, SEG + 1, 3 * SEG, 10_000] {
            let values: Vec<u32> = (0..n).map(|_| rng.next_u32() % 2).collect();
            let (a, ta) = exclusive_scan_segmented(&values);
            let (b, tb) = exclusive_scan_reference(&values);
            assert_eq!(a, b, "n={n} 分段分解 ≠ 参考实现");
            assert_eq!(ta, tb);
        }
    }

    #[test]
    fn segmented_matches_reference_on_histogram_domain() {
        // 排序 spine 消费域:256 bin × nseg 直方图计数(单值可 > 1)。
        let mut rng = Pcg32::new(11, 54);
        let values: Vec<u32> = (0..4096).map(|_| rng.next_u32() % 300).collect();
        let (a, ta) = exclusive_scan_segmented(&values);
        let (b, tb) = exclusive_scan_reference(&values);
        assert_eq!(a, b);
        assert_eq!(ta, tb);
    }

    #[test]
    fn spine_total_slot_is_sum() {
        let values = vec![1u32; 700];
        let nseg = values.len().div_ceil(SEG);
        let sp = spine(&seg_sums(&values, nseg));
        assert_eq!(sp.len(), nseg + 1);
        assert_eq!(sp[nseg], 700);
    }
}
