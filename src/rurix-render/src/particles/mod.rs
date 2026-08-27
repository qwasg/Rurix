//! G35 GPU 粒子系统(RFC-0049;事实源 = milestones/g35/G35_CONTRACT.md)。
//!
//! 对标并超越 UE5 Niagara 的五轴(G35_PLAN §1):确定性 / 光追集成 / 规模 /
//! 流体统一物理 / 数据驱动作者面。本模块 = host 金标准层(全 safe 可单测,
//! `forbid(unsafe_code)` 随 crate);device 面 = `kernels/g35_*.rx`(rurixc
//! --target vulkan 产 SPV),经 `rurix_rt::vk::run_compute`(probe)或
//! `DeviceFrameSession`(生产车道)派发。
//!
//! ## G35-P 冻结契约 v1(波内共享;改动走契约修订)
//!
//! ### 确定性协议(对 Niagara GPU sim 非确定性的根因解法)
//! - **禁原子抢槽**:发射/压缩槽位一律经**分段稳定 scan**(下述三 kernel)
//!   推导,顺序 = 粒子下标序,与线程调度无关 ⇒ 固定输入双跑位级一致。
//! - **随机带单源纪律**(G28 RFC-0045 §1.2 同律):PCG32 只在 host 出现
//!   ([`rand_table`]),device 经 SSBO 只读消费
//!   `r = rand_table[(pid·RAND_K + slot) % RAND_TABLE_LEN]`;device 端
//!   零超越函数、零位运算(kernel 语言面无 `^`/`>>`,整数域一律 usize
//!   除/模精确算术,g34_unified_gi.rx 图集 unpack 先例)。
//! - **容差协议**:整数流(pid/flags/scan/sort 键与序)device↔host **零容差
//!   位级**;f32 流(pos/vel/age)device↔host 走标定容差(measured×2.0 协议
//!   冻结,程序产禁手写,g35_budget.json);device 双跑一律位级。
//!
//! ### 分段布局(SoA)
//! - `SEG` = 256:所有池容量 N 须为 SEG 整倍数;nseg = N/SEG ≤ [`NSEG_MAX`]。
//! - 粒子流 SoA 各占独立 SSBO:pos_x/y/z、vel_x/y/z、age、life(f32);
//!   pid、alive_flags(u32)。压缩 = ping-pong 双组。
//!
//! ### 分段稳定 exclusive scan 三 kernel(本文件 [`scan`] host 金标准同源)
//! 无 shared memory / 无原子 / 无 lookback(Vulkan 前进保证缺位,保守臂为
//! 生产形态;Onesweep/decoupled-lookback 为实验臂,RFC-0049 §9 裁决):
//! 1. `g35_scan_seg_sum.rx`  dispatch [nseg,1,1]:线程 s 串行求段和
//!    → seg_sums[s];
//! 2. `g35_scan_spine.rx`    dispatch [1,1,1]:单 invocation 对 seg_sums
//!    串行 exclusive scan → seg_offsets[0..nseg],并写总和到
//!    seg_offsets[nseg](槽位 = 存活总数,indirect args 消费面);
//! 3. `g35_scan_seg_apply.rx` dispatch [nseg,1,1]:线程 s 段内串行
//!    running 前缀 → out[i] = 全局 exclusive scan。
//! params 面(f32 SSBO):`[0]=n [1]=nseg [2..4)=reserved(恒 0)`
//! (n ≤ 2^24 f32 精确)。元素 u32,单值 < 2^24、总和 < 2^32。
//!
//! ### 深度排序键(G35-1 排序基元消费面)
//! 零位转换(语言面无 bitcast):`key = floor(clamp(d/d_max,0,1)·16777215)`
//! 24 位量化单调键([`depth_key24`]),radix 3 pass × 8 bit,digit =
//! `(key as usize / 256^p) % 256`。稳定序 = 段序×段内序。
//!
//! ### 波次文件面(所有权分区,并行纪律)
//! - 本文件 + [`scan`] + `kernels/g35_scan_*.rx`:契约与 scan 基元(冻结)。
//! - [`primitives`](G35-1):radix sort(hist/spine/scatter)+ compact_u32
//!   基元 + `bin/g35_primitives_device.rs` + `ci/g35_primitives_smoke.py`。
//! - [`core`](G35-2):粒子池/发射/积分/稳定压缩/间接参数 +
//!   `bin/g35_particle_core_device.rs` + `ci/g35_particle_core_smoke.py`。

pub mod collision;
pub mod core;
pub mod emitter_asset;
pub mod events;
pub mod fluid;
pub mod oit_arms;
pub mod primitives;
pub mod replay;
pub mod scan;

/// 分段长度(线程 = 段;段内串行确定序)。
pub const SEG: usize = 256;
/// v1 段数上限(spine 单 invocation 串行域;1M 粒子 = 4096 段)。
pub const NSEG_MAX: usize = 4096;
/// v1 池容量上限 = SEG × NSEG_MAX。
pub const PARTICLE_CAP_MAX: usize = SEG * NSEG_MAX;
/// 随机表长(host Pcg32 单源;device 只读消费)。
pub const RAND_TABLE_LEN: usize = 65536;
/// 随机表步进素数(pid 通道去相关)。
pub const RAND_K: usize = 7919;
/// 24 位深度键满刻度(2^24 − 1;f32 精确表示域)。
pub const DEPTH_KEY_MAX: f32 = 16_777_215.0;

/// PCG32(O'Neill 2014;state/inc 显式,与 seed 一一对应)——随机带单源
/// 纪律的 host 端唯一随机源。输出 u32;[`rand_table`] 折算 [0,1) f32。
#[derive(Clone)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    /// PCG-XSH-RR 标准初始化(O'Neill 参考实现字面)。
    pub fn new(seed: u64, stream: u64) -> Self {
        let mut s = Self {
            state: 0,
            inc: (stream << 1) | 1,
        };
        s.next_u32();
        s.state = s.state.wrapping_add(seed);
        s.next_u32();
        s
    }

    /// 下一 u32(PCG-XSH-RR 输出置换)。
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// [0,1) f32(高 24 位 ÷ 2^24——f32 精确域,device 消费同位型)。
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }
}

/// 随机带单源:长度 [`RAND_TABLE_LEN`] 的 [0,1) f32 表(seed 全定)。
/// device 消费律:`rand_table[(pid·RAND_K + slot) % RAND_TABLE_LEN]`。
pub fn rand_table(seed: u64) -> Vec<f32> {
    let mut rng = Pcg32::new(seed, 54);
    (0..RAND_TABLE_LEN).map(|_| rng.next_f32()).collect()
}

/// 24 位量化深度排序键(单调;host/device 同式零位转换)。
/// `d_max ≤ 0` 视为退化域,一律返回 0(调用方保证正规化)。
pub fn depth_key24(depth: f32, d_max: f32) -> u32 {
    if d_max <= 0.0 {
        return 0;
    }
    let t = (depth / d_max).clamp(0.0, 1.0);
    (t * DEPTH_KEY_MAX).floor() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcg32_deterministic_and_seed_sensitive() {
        let a: Vec<u32> = {
            let mut r = Pcg32::new(42, 54);
            (0..8).map(|_| r.next_u32()).collect()
        };
        let b: Vec<u32> = {
            let mut r = Pcg32::new(42, 54);
            (0..8).map(|_| r.next_u32()).collect()
        };
        let c: Vec<u32> = {
            let mut r = Pcg32::new(43, 54);
            (0..8).map(|_| r.next_u32()).collect()
        };
        assert_eq!(a, b, "同 seed 双跑必须位级一致");
        assert_ne!(a, c, "异 seed 必须可分辨");
    }

    #[test]
    fn rand_table_domain_and_determinism() {
        let t1 = rand_table(7);
        let t2 = rand_table(7);
        assert_eq!(t1.len(), RAND_TABLE_LEN);
        assert_eq!(t1, t2);
        assert!(t1.iter().all(|v| (0.0..1.0).contains(v)));
    }

    #[test]
    fn depth_key24_monotone_and_bounded() {
        let d_max = 100.0;
        let mut prev = 0u32;
        for i in 0..=1000 {
            let d = i as f32 * 0.1;
            let k = depth_key24(d, d_max);
            assert!(k >= prev, "键必须单调不减");
            assert!(k <= DEPTH_KEY_MAX as u32);
            prev = k;
        }
        assert_eq!(depth_key24(-1.0, d_max), 0);
        assert_eq!(depth_key24(1000.0, d_max), DEPTH_KEY_MAX as u32);
        assert_eq!(depth_key24(1.0, 0.0), 0, "退化域一律 0");
    }
}
